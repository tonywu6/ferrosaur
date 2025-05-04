export const todos = todoList();

function todoList() {
  const items = [];

  const create = () => {
    const todo = new Todo();
    items.push(todo);
    return todo;
  };

  return { create };
}

class Todo {
  done = false;
}
