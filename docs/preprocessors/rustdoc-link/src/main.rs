use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use async_lsp::LanguageServer;
use lsp_types::Position;
use mdbook::{book::Book, preprocess::PreprocessorContext, BookItem};
use serde::Deserialize;
use tap::{Pipe, Tap, TapFallible, TapOptional};
use tokio::task::JoinSet;

use crate::{
    ast::ItemName,
    client::{document_position, Client, ExternalDocLinks, ExternalDocs},
    markdown::{markdown_parser, MarkdownStream},
};

#[derive(clap::Parser, Deserialize, Debug, Default, Clone)]
#[serde(rename_all = "kebab-case")]
struct BuildOptions {
    #[arg(long)]
    #[serde(default)]
    manifest_dir: Option<PathBuf>,
    #[arg(long)]
    #[serde(default)]
    pub smart_punctuation: bool,
    #[arg(long)]
    #[serde(default)]
    pub prefer_local_links: bool,
}

#[derive(clap::Parser, Debug, Clone)]
struct Command {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Commands {
    Supports { renderer: String },
    Markdown(BuildOptions),
}

#[tokio::main]
async fn main() -> Result<()> {
    use clap::Parser;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    match Command::parse().command {
        Some(Commands::Supports { .. }) => Ok(()),
        Some(Commands::Markdown(options)) => markdown(options).await,
        None => mdbook().await,
    }
}

async fn mdbook() -> Result<()> {
    let (context, mut book): (PreprocessorContext, Book) = Vec::new()
        .pipe(|mut buf| std::io::stdin().read_to_end(&mut buf).and(Ok(buf)))?
        .pipe(String::from_utf8)?
        .pipe_as_ref(serde_json::from_str)?;

    let options = {
        let mut options = if let Some(config) = context.config.get_preprocessor(preprocessor_name())
        {
            BuildOptions::deserialize(toml::Value::Table(config.clone()))?
        } else {
            Default::default()
        };
        if let Some(path) = options.manifest_dir {
            options.manifest_dir = Some(context.root.join(path))
        } else {
            options.manifest_dir = Some(context.root)
        }
        options.smart_punctuation = context
            .config
            .get_deserialized_opt::<bool, _>("output.html.smart-punctuation")
            .unwrap_or_default()
            .unwrap_or(true);
        options
    };

    let (client, main) = Client::spawn(options).await?;

    let mut tasks = JoinSet::new();

    // FIXME: collect all items before request
    // FIXME: restore links that cannot be resolved

    book.iter().for_each(|item| {
        let BookItem::Chapter(ch) = item else { return };
        let Some(key) = &ch.source_path else { return };
        let key = key.clone();
        let content = ch.content.clone();
        let smart_punctuation = client.config.build_opts.smart_punctuation;
        let mut client = client.clone();
        tasks.spawn(async move {
            let stream = markdown_parser(&content, smart_punctuation);
            let output = client.process(stream).await?;
            Ok::<_, anyhow::Error>((key, output))
        });
    });

    let mut output = tasks
        .join_all()
        .await
        .into_iter()
        .filter_map(|result| match result {
            Ok(output) => Some(output),
            Err(error) => {
                tracing::warn!("{error:?}");
                None
            }
        })
        .collect::<HashMap<_, _>>();

    book.for_each_mut(|item| {
        let BookItem::Chapter(ch) = item else { return };
        let Some(key) = &ch.source_path else { return };
        if let Some(content) = output.remove(key) {
            ch.content = content;
        }
    });

    client.close(main).await?;

    serde_json::to_string(&book)?.pipe(|out| std::io::stdout().write_all(out.as_bytes()))?;

    Ok(())
}

async fn markdown(options: BuildOptions) -> Result<()> {
    let (mut client, main) = Client::spawn(options).await?;

    let stream = Vec::new()
        .pipe(|mut buf| std::io::stdin().read_to_end(&mut buf).and(Ok(buf)))?
        .pipe(String::from_utf8)?;

    let stream = markdown_parser(&stream, client.config.build_opts.smart_punctuation);

    let output = client.process(stream).await?;

    std::io::stdout().write_all(output.as_bytes())?;

    client.close(main).await?;

    Ok(())
}

impl Client {
    async fn process(&mut self, stream: MarkdownStream<'_>) -> Result<String> {
        use pulldown_cmark::{CowStr, Event, Tag};
        use pulldown_cmark_to_cmark::cmark;

        let mut request = vec![];

        let buffer = stream
            .inspect(|event| {
                if let Event::Start(Tag::Link { dest_url, .. }) = &event {
                    request.push(dest_url.to_string());
                }
            })
            .collect::<Vec<_>>();

        tracing::debug!(target: "Client::process", "{request:#?}");

        let links = self.resolve(request).await?;

        tracing::debug!(target: "Client::process", "{links:#?}");

        let update_link = |link: &mut CowStr<'_>| {
            let url = links
                .get(link.as_ref())
                .and_then(|links| {
                    if self.config.build_opts.prefer_local_links {
                        links.local.as_ref()
                    } else {
                        links.web.as_ref()
                    }
                })
                .map(|u| u.as_str());
            if let Some(url) = url {
                *link = url.to_owned().into();
            }
        };

        let stream = buffer.into_iter().map(|mut event| {
            if let Event::Start(Tag::Link { dest_url, .. }) = &mut event {
                update_link(dest_url);
            }
            event
        });

        String::new()
            .pipe(|mut wr| cmark(stream, &mut wr).and(Ok(wr)))?
            .pipe(Ok)
    }

    async fn resolve(&mut self, request: Vec<String>) -> Result<ItemLinks> {
        use lsp_types::{
            DidCloseTextDocumentParams, DidOpenTextDocumentParams, TextDocumentIdentifier,
            TextDocumentItem,
        };

        let request = ItemRequestBatch::new(&self.config.entrypoint.src, request);

        let mut collected = HashMap::new();

        if request.request.is_empty() {
            return Ok(collected);
        }

        self.server.did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: self.config.entrypoint.url.clone(),
                text: request.context,
                language_id: "rust".into(),
                version: 1,
            },
        })?;

        self.stabilizer.wait().await;

        let mut tasks = JoinSet::new();

        for ItemRequest {
            path,
            hash,
            position,
        } in &request.request
        {
            if collected.contains_key(path) {
                continue;
            }

            let server = self.server.clone();
            let uri = self.config.entrypoint.url.clone();
            let pos = *position;
            let path = path.clone();
            let hash = hash.clone();

            tasks.spawn(async move {
                let ExternalDocLinks { web, local } = server
                    .request::<ExternalDocs>(document_position(uri, pos))
                    .await
                    .tap_err(|err| tracing::warn!(target: "ExternalDocs", "{err:#?}"))
                    .unwrap_or_default()?;

                let (web, local) = if let Some(hash) = hash.as_deref() {
                    let web = web.tap_some_mut(|u| u.set_fragment(Some(hash)));
                    let local = local.tap_some_mut(|u| u.set_fragment(Some(hash)));
                    (web, local)
                } else {
                    (web, local)
                };

                if web.is_none() && local.is_none() {
                    None
                } else {
                    let links = ExternalDocLinks { web, local };
                    let key = if let Some(hash) = hash {
                        format!("{path}#{hash}")
                    } else {
                        path
                    };
                    Some((key, links))
                }
            });
        }

        while let Some(res) = tasks.join_next().await {
            if let Ok(Some((key, links))) = res {
                collected.insert(key, links);
            };
        }

        self.server.did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier {
                uri: self.config.entrypoint.url.clone(),
            },
        })?;

        Ok(collected)
    }
}

#[derive(Debug)]
struct ItemRequestBatch {
    context: String,
    request: Vec<ItemRequest>,
}

#[derive(Debug)]
struct ItemRequest {
    path: String,
    hash: Option<String>,
    position: Position,
}

impl ItemRequestBatch {
    fn new(source: &str, items: Vec<String>) -> Self {
        use syn::parse::{Parse, Parser};

        let source = format!("{source}\nfn __6c0db446e2fa428eb93e3c71945e9654() {{\n");

        let mut request = vec![];
        let mut line = source.chars().filter(|&c| c == '\n').count();

        let context = HashSet::<String>::from_iter(items)
            .into_iter()
            .filter_map(|name| {
                let mut name = name.split('#');
                let path = name.next().unwrap();
                let item = ItemName::parse.parse_str(path).ok()?;
                let position = item.ident().span().start();
                if position.line == 1 {
                    let path = path.to_owned();
                    let hash = name.next().map(ToOwned::to_owned);
                    Some((path, hash, position.column))
                } else {
                    None
                }
            })
            .fold(source, |mut output, (path, hash, column)| {
                use std::fmt::Write;
                let _ = writeln!(output, "{path};");
                let position = Position::new(line as _, column as _);
                request.push(ItemRequest {
                    path,
                    hash,
                    position,
                });
                line += 1;
                output
            });

        let context = context.tap_mut(|c| c.push('}'));

        Self { context, request }
    }
}

type ItemLinks = HashMap<String, client::ExternalDocLinks>;

fn preprocessor_name() -> &'static str {
    let name = env!("CARGO_PKG_NAME");
    if let Some(idx) = name.find('-') {
        &name[idx + 1..]
    } else {
        name
    }
}

mod client {
    use std::{ops::ControlFlow, path::PathBuf, process::Stdio, task::Poll, time::Duration};

    use anyhow::{bail, Context, Result};
    use async_lsp::{
        concurrency::ConcurrencyLayer, panic::CatchUnwindLayer, router::Router,
        tracing::TracingLayer, LanguageServer, MainLoop, ServerSocket,
    };
    use lsp_types::{
        notification::{Progress, PublishDiagnostics, ShowMessage},
        request::Request,
        ClientCapabilities, GeneralClientCapabilities, InitializeParams, InitializedParams,
        NumberOrString, Position, PositionEncodingKind, ProgressParams, ProgressParamsValue,
        TextDocumentIdentifier, TextDocumentPositionParams, Url, WindowClientCapabilities,
        WorkDoneProgress, WorkspaceFolder,
    };
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    use tokio::{process::Command, sync::mpsc, task::JoinHandle};
    use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
    use tower::ServiceBuilder;

    use crate::{metadata::Entrypoint, sync::Debounce, BuildOptions};

    #[derive(Debug, Clone)]
    pub struct Environment {
        pub root_dir: PathBuf,
        pub entrypoint: Entrypoint,
        pub build_opts: BuildOptions,
    }

    impl Environment {
        fn new(build_opts: BuildOptions) -> Result<Self> {
            let root_dir = build_opts
                .manifest_dir
                .clone()
                .map(Ok)
                .unwrap_or_else(std::env::current_dir)?;

            let entrypoint = Entrypoint::new(&root_dir)?;

            Ok(Self {
                root_dir,
                entrypoint,
                build_opts,
            })
        }
    }

    #[derive(Debug, Clone)]
    pub struct Client {
        pub server: ServerSocket,
        pub config: Environment,
        pub stabilizer: Debounce,
    }

    impl Client {
        pub async fn spawn(options: BuildOptions) -> Result<(Self, JoinHandle<()>)> {
            let config = Environment::new(options)?;

            let (tx, rx) = mpsc::channel(16);

            let stabilizer = Debounce::new(rx, Duration::from_secs(2));

            let (background, mut server) = MainLoop::new_client(move |_| {
                struct State {
                    tx: mpsc::Sender<Poll<()>>,
                }

                let state = State { tx };

                let mut router = Router::new(state);

                router
                    .notification::<Progress>(|state, progress| {
                        tracing::debug!(target: "Progress", "{progress:#?}");

                        if indexing_begin(&progress) {
                            let tx = state.tx.clone();
                            tokio::spawn(async move { tx.send(Poll::Pending).await.ok() });
                        }

                        if indexing_end(&progress) {
                            let tx = state.tx.clone();
                            tokio::spawn(async move { tx.send(Poll::Ready(())).await.ok() });
                        }

                        ControlFlow::Continue(())
                    })
                    .notification::<PublishDiagnostics>(|_, diagnostics| {
                        tracing::debug!(target: "PublishDiagnostics", "{diagnostics:#?}");
                        ControlFlow::Continue(())
                    })
                    .notification::<ShowMessage>(|_, message| {
                        tracing::debug!(target: "ShowMessage", "{message:#?}");
                        ControlFlow::Continue(())
                    })
                    .event(|_, _: StopEvent| ControlFlow::Break(Ok(())));

                ServiceBuilder::new()
                    .layer(TracingLayer::default())
                    .layer(CatchUnwindLayer::default())
                    .layer(ConcurrencyLayer::default())
                    .service(router)
            });

            let proc = Command::new("rust-analyzer")
                .current_dir(&config.root_dir)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::inherit())
                .kill_on_drop(true)
                .spawn()
                .context("failed to spawn rust-analyzer")?;

            let background = tokio::spawn(async move {
                let mut proc = proc;
                let stdout = proc.stdout.take().unwrap();
                let stdin = proc.stdin.take().unwrap();
                background
                    .run_buffered(stdout.compat(), stdin.compat_write())
                    .await
                    .unwrap();
            });

            let root_uri = Url::from_directory_path(&config.root_dir).unwrap();

            let init = server
                .initialize(InitializeParams {
                    workspace_folders: Some(vec![WorkspaceFolder {
                        uri: root_uri.clone(),
                        name: "root".into(),
                    }]),
                    capabilities: ClientCapabilities {
                        experimental: Some(json! {{
                            "localDocs": true,
                        }}),
                        window: Some(WindowClientCapabilities {
                            work_done_progress: Some(true),
                            ..Default::default()
                        }),
                        general: Some(GeneralClientCapabilities {
                            position_encodings: Some(vec![PositionEncodingKind::UTF8]),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .await?;

            tracing::debug!(target: "Initialize", "{init:#?}");

            if init.capabilities.position_encoding != Some(PositionEncodingKind::UTF8) {
                bail!("this rust-analyzer does not support utf-8 positions")
            }

            server.initialized(InitializedParams {})?;

            let client = Self {
                server,
                config,
                stabilizer,
            };

            Ok((client, background))
        }

        pub async fn close(self, background: JoinHandle<()>) -> Result<()> {
            let Self { mut server, .. } = self;
            server.shutdown(()).await?;
            server.exit(())?;
            server.emit(StopEvent)?;
            background.await?;
            Ok(())
        }
    }

    pub enum ExternalDocs {}

    impl Request for ExternalDocs {
        const METHOD: &'static str = "experimental/externalDocs";
        type Params = TextDocumentPositionParams;
        type Result = Option<ExternalDocLinks>;
    }

    #[derive(Debug, Deserialize, Serialize)]
    pub struct ExternalDocLinks {
        pub web: Option<Url>,
        pub local: Option<Url>,
    }

    struct StopEvent;

    pub fn document_position(uri: Url, position: Position) -> TextDocumentPositionParams {
        TextDocumentPositionParams {
            text_document: TextDocumentIdentifier { uri },
            position,
        }
    }

    fn indexing_begin(progress: &ProgressParams) -> bool {
        matches!(progress, ProgressParams {
            token: NumberOrString::String(token),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(_)),
        } if token == "rustAnalyzer/Indexing")
    }

    fn indexing_end(progress: &ProgressParams) -> bool {
        matches!(progress, ProgressParams {
            token: NumberOrString::String(token),
            value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(_)),
        } if token == "rustAnalyzer/Indexing")
    }
}

mod metadata {
    use std::path::Path;

    use anyhow::{anyhow, Context, Result};
    use cargo_toml::{Manifest, Product};
    use lsp_types::Url;

    #[derive(Debug, Clone)]
    pub struct Entrypoint {
        pub url: Url,
        pub src: String,
    }

    impl Entrypoint {
        pub fn new<P: AsRef<Path>>(from_dir: P) -> Result<Self> {
            let mut dir = from_dir.as_ref();

            let path = loop {
                let path = dir.join("Cargo.toml");
                if path.exists() {
                    break path;
                }
                dir = match dir.parent() {
                    Some(dir) => dir,
                    None => {
                        return Err(anyhow!(from_dir.as_ref().display().to_string()))
                            .context("failed to find a Cargo.toml");
                    }
                };
            };

            let manifest = {
                let mut manifest = Manifest::from_path(&path)?;
                manifest.complete_from_path(&path)?;
                manifest
            };

            let root_url = Url::from_file_path(&path).unwrap();

            let url = if let Some(Product {
                path: Some(lib), ..
            }) = manifest.lib
            {
                Ok(root_url.join(&lib)?)
            } else if let Some(bin) = manifest.bin.iter().find_map(|bin| bin.path.as_ref()) {
                Ok(root_url.join(bin)?)
            } else {
                Err(anyhow!("{}", path.display()))
                    .context("Cargo.toml does not have a lib or bin target")
            }?;

            let src = std::fs::read_to_string(url.path())?;

            Ok(Self { url, src })
        }
    }
}

mod ast {
    use syn::{
        parse::{Parse, ParseStream},
        spanned::Spanned,
        Error, Expr, ExprCall, ExprMacro, ExprPath, Ident, Macro, Path, Result,
    };
    use tap::Pipe;

    pub enum ItemName {
        Path { path: Path },
        Call { path: Path },
        Macro { mac: Macro },
    }

    impl Parse for ItemName {
        fn parse(input: ParseStream) -> Result<Self> {
            let expr = Expr::parse(input)?;
            match expr {
                Expr::Path(ExprPath {
                    path, qself: None, ..
                }) => Ok(Self::Path { path }),

                Expr::Call(ExprCall { func, .. }) => match *func {
                    Expr::Path(ExprPath {
                        path, qself: None, ..
                    }) => Ok(Self::Call { path }),
                    func => Error::new(func.span(), "expected a path").pipe(Err),
                },

                Expr::Macro(ExprMacro { mac, .. }) => Ok(Self::Macro { mac }),

                expr => Error::new(expr.span(), "expected a path, call, or macro").pipe(Err),
            }
        }
    }

    impl ItemName {
        pub fn ident(&self) -> &Ident {
            let path = match &self {
                Self::Path { path } => path,
                Self::Call { path, .. } => path,
                Self::Macro { mac } => &mac.path,
            };
            &path
                .segments
                .last()
                .expect("path should not be empty")
                .ident
        }
    }

    #[cfg(test)]
    const _: () = {
        use proc_macro2::TokenStream;
        use quote::{quote, ToTokens};

        impl ToTokens for ItemName {
            fn to_tokens(&self, tokens: &mut TokenStream) {
                match self {
                    Self::Path { path } => path.to_tokens(tokens),
                    Self::Call { path, .. } => quote! { #path () }.to_tokens(tokens),
                    Self::Macro { mac } => mac.to_tokens(tokens),
                }
            }
        }
    };
}

mod markdown {
    use pulldown_cmark::{BrokenLink, BrokenLinkCallback, CowStr, Event, Options, Parser};
    use tap::Pipe;

    fn options(smart_punctuation: bool) -> Options {
        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);
        if smart_punctuation {
            opts.insert(Options::ENABLE_SMART_PUNCTUATION);
        }
        opts
    }

    pub type MarkdownStream<'a> = Parser<'a, BrokenLinks>;

    pub fn markdown_parser(text: &str, smart_punctuation: bool) -> MarkdownStream<'_> {
        Parser::new_with_broken_link_callback(text, options(smart_punctuation), Some(BrokenLinks))
    }

    pub struct BrokenLinks;

    impl<'input> BrokenLinkCallback<'input> for BrokenLinks {
        fn handle_broken_link(
            &mut self,
            link: BrokenLink<'input>,
        ) -> Option<(CowStr<'input>, CowStr<'input>)> {
            let inner = if let CowStr::Borrowed(inner) = link.reference {
                let parse = markdown_parser(inner, false);

                let inner = parse
                    .filter_map(|event| match event {
                        Event::Text(inner) => Some(inner),
                        Event::Code(inner) => Some(inner),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                if inner.len() == 1 {
                    inner.into_iter().next().unwrap()
                } else {
                    inner
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Box<str>>()
                        .pipe(CowStr::Boxed)
                }
            } else {
                link.reference.clone()
            };
            if inner.is_empty() {
                None
            } else {
                Some((inner, link.reference))
            }
        }
    }
}

mod sync {
    use std::{
        sync::{Arc, Mutex, RwLock},
        task::Poll,
        time::Duration,
    };

    use tokio::{
        sync::{mpsc, oneshot},
        time::sleep,
    };

    #[derive(Debug, Clone)]
    pub struct Debounce {
        state: State,
        queue: Queue,
    }

    type State = Arc<RwLock<Poll<()>>>;
    type Queue = Arc<Mutex<Vec<oneshot::Sender<()>>>>;

    impl Debounce {
        pub fn new(mut rx: mpsc::Receiver<Poll<()>>, wait: Duration) -> Self {
            let queue = Queue::default();
            let state = Arc::new(RwLock::new(Poll::Pending));

            tokio::spawn({
                let queue = queue.clone();
                let state = state.clone();
                async move {
                    let mut abort = None;
                    while let Some(event) = rx.recv().await {
                        if event.is_ready() {
                            let queue = queue.clone();
                            abort = Some(tokio::spawn(async move {
                                sleep(wait).await;
                                let queue = std::mem::take(&mut *queue.lock().unwrap());
                                for tx in queue {
                                    tx.send(()).unwrap()
                                }
                            }));
                        } else {
                            if let Some(abort) = abort.take() {
                                abort.abort();
                            }
                            *state.write().unwrap() = Poll::Pending;
                        }
                    }
                }
            });

            Self { queue, state }
        }

        pub async fn wait(&self) {
            if self.state.read().unwrap().is_pending() {
                let (tx, rx) = oneshot::channel();
                self.queue.lock().unwrap().push(tx);
                rx.await.unwrap();
            }
        }
    }
}
