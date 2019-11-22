use mogwai::prelude::*;

use super::todo::Todo;


pub enum TodoListMsg {
  AddTodo(String),
  RemoveTodo(usize),
  CompleteTodo(usize),
  UncompleteTodo(usize),
  EditedTodo(usize, String)
}


pub struct TodoList {
  next_index: usize,
  todos: Vec<Todo>,
  tx_msg: Transmitter<TodoListMsg>,
  gizmo: Gizmo
}

pub fn mk_bool_to_display() -> impl Fn(&bool) -> Option<String> {
  |should| {
    trace!("should display: {}", should);
    if *should {
      Some("block".to_string())
    } else {
      Some("none".to_string())
    }
  }
}


impl TodoList {
  /// Creates a new TodoList, which manages its own transmitter for any
  /// updates.
  pub fn new(
    tx_msg: Transmitter<TodoListMsg>,
    rx_display: Receiver<bool>
  ) -> TodoList {
    let gizmo =
      ul()
      .class("todo-list")
      .rx_style(
        "display",
        "none",
        rx_display.branch_map(mk_bool_to_display())
      )
      .build()
      .unwrap();

    TodoList {
      gizmo,
      tx_msg,
      next_index: 0,
      todos: vec![],
    }
  }

  pub fn add_todo(&mut self, name: String) {
    let todo =
      Todo::new(self.next_index, name, self.tx_msg.clone());

    utils::nest_gizmos(&self.gizmo, &todo.gizmo)
      .unwrap();

    self
      .todos
      .push(todo);

    self.next_index += 1;

  }

  pub fn remove_todo(&mut self, index: usize) {
    self
      .todos
      .retain(|item| item.index != index);
  }
}


fn todo_list(
  mut rx_todo: Receiver<String>,
  mut tx_display: Transmitter<bool>,
  tx_num_items: Transmitter<usize>
) -> HtmlElement {
  // Create a pair of terminals for all our todo list messages
  let (tx_msg, mut rx_msg) = terminals();

  // Create a pair of terminals to tell the list when to display
  let rx_display = tx_display.spawn_recv();

  // Create a new mutable TodoList,
  let mut list = TodoList::new(tx_msg.clone(), rx_display);
  // get its html_element
  let el = list.gizmo.html_element.clone();

  // Forward rx_todo into tx_msg
  rx_todo.forward_map(
    tx_msg,
    |name| Some(TodoListMsg::AddTodo(name.to_string()))
  );

  // Set our responder to all incoming TodoListMsgs
  rx_msg.set_responder(move |msg| {
    let num_todos_before =
      list.todos.len();

    use TodoListMsg::*;
    match msg {
      AddTodo(name) => {
        list.add_todo(name.to_string());
      }
      RemoveTodo(index) => {
        list.remove_todo(*index);
      }
      CompleteTodo(index) => {
        list
          .todos
          .get_mut(*index)
          .into_iter()
          .for_each(|todo| {
            todo.is_done = true;
          });
      }
      UncompleteTodo(index) => {
        list
          .todos
          .get_mut(*index)
          .into_iter()
          .for_each(|todo| {
            todo.is_done = false;
          });
      }
      EditedTodo(index, name) => {
        list
          .todos
          .get_mut(*index)
          .into_iter()
          .for_each(|todo| {
            todo.name = name.clone();
          });
      }
    }

    let num_todos_after =
      list.todos.len();

    if num_todos_before == 0 && num_todos_after == 1 {
      tx_display.send(&true);
    } else if num_todos_after == 0 && num_todos_before == 1 {
      tx_display.send(&false);
    }

    if num_todos_before != num_todos_after {
      tx_num_items.send(&num_todos_after);
    }
  });

  // Return just the pre-built HtmlElement and let the todo list
  // fend for itself
  el
}


/// This is the section that holds our todo list
pub fn todo_main_section(
  tx_display: Transmitter<bool>,
  rx_todo: Receiver<String>,
  tx_num_items: Transmitter<usize>,
) -> GizmoBuilder {
  // Wire the display tx to a style rx to hide the footer
  let rx_display = Receiver::<String>::new();
  tx_display
    .clone()
    .wire_map(&rx_display, mk_bool_to_display());

  section()
    .class("main")
    .rx_style("display", "none", rx_display)
    .with(
      // This is the "check all as complete" toggle
      input()
        .attribute("id", "toggle-all")
        .attribute("type", "checkbox")
        .class("toggle-all")
    )
    .with(
      label()
        .attribute("for", "toggle-all")
        .text("Mark all as complete")
    )
    .with_pre_built(todo_list(rx_todo, tx_display, tx_num_items))
}
