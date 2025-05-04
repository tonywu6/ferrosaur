export declare const todos: TodoList;

interface TodoList {
  create: () => Todo;
}

interface Todo {
  done: boolean;
}
