use mogwai::prelude::*;

use super::todo_list::TodoListMsg;
use super::utils;


pub struct Todo {
  pub gizmo: Gizmo,
  pub index: usize,
  pub name: String,
  pub is_done: bool,
}


impl Todo {
  pub fn new(index: usize, name: String, tx: Transmitter<TodoListMsg>) -> Todo {
    let gizmo_bldr =
      todo_item(index, name.clone(), tx);
    let gizmo =
      gizmo_bldr
      .build()
      .unwrap();
    let todo =
      Todo {
        index,
        name,
        is_done: false,
        gizmo
      };
    todo
  }
}


/// Creates a new todo gizmo and also returns a tx that sends messages to
/// to a todo list.
pub fn todo_item(index:usize, name:String, tx_msg: Transmitter<TodoListMsg>) -> GizmoBuilder {
  // We will have a button to click to remove the todo
  let (tx_remove, mut rx_remove) = terminals();
  // Wire the receiver of clicks to send messages into tx_msg
  rx_remove.forward_map(
    tx_msg.clone(),
    move |_| Some(TodoListMsg::RemoveTodo(index))
  );

  // We will have a taggle input to toggle todo completion
  let mut tx_toggle = Transmitter::new();
  let rx_toggle = Receiver::<bool>::new();
  let tx_msg_complete = tx_msg.clone();
  tx_toggle.wire_fold(
    &rx_toggle,
    false,
    move |is_complete:&bool, _| {
      let next =
        // We'll also send out to the todolist
        if *is_complete {
          tx_msg_complete.send(&TodoListMsg::UncompleteTodo(index));
          false
        } else {
          tx_msg_complete.send(&TodoListMsg::CompleteTodo(index));
          true
        };

      (next, Some(next))
    }
  );

  // Turn the completion toggle state into a class
  let rx_class =
    rx_toggle.branch_map(|is_complete| {
      Some(
        if *is_complete {
          "completed"
        } else {
          ""
        }.to_string()
      )
    });

  // The name of this todo can be edited, so we'll need a tx rx for that.
  let (tx_todo_name, rx_todo_name) = terminals();

  // And we'll need some tx rx pairs for editing
  let (tx_start_edit, mut rx_start_edit) = terminals();
  let (tx_end_edit, mut rx_end_edit) = terminals();

  // When we start the editing process we'll send "editing" to our rx_class
  // When we end the editing process we'll send "" to rx_class the value of our todo to our rx_name
  let tx_class = rx_class.new_trns();
  let tx_class_start = tx_class.clone();
  rx_start_edit.set_responder(move |_| {
    tx_class_start.send(&"editing".into());
  });

  let tx_msg_end_edit = tx_msg.clone();
  rx_end_edit.set_responder(move |ev:&Event| {
    let value =
      utils::event_input_value(ev)
      .unwrap();
    // Send the class name
    tx_class.send(&"".into());
    tx_todo_name.send(&value);
    tx_msg_end_edit.send(&TodoListMsg::EditedTodo(index, value));
  });


  li()
    .rx_class("", rx_class)
    .tx_on("dblclick", tx_start_edit)
    .with(
      div()
        .class("view")
        .with(
          input()
            .class("toggle")
            .attribute("type", "checkbox")
            .style("cursor", "pointer")
            .tx_on("click", tx_toggle)
        )
        .with(
          label()
            .rx_text(&name, rx_todo_name)
        )
        .with(
          button()
            .class("destroy")
            .style("cursor", "pointer")
            .tx_on("click", tx_remove)
        )
    )
    .with(
      input()
        .class("edit")
        .value(&name)
        .tx_on("focusout", tx_end_edit.clone())
        .tx_on("change", tx_end_edit)
    )
}
