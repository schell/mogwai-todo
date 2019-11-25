use std::sync::{Arc, Mutex};
use mogwai::prelude::*;
use web_sys::KeyboardEvent;

use super::todo_list::TodoListMsg;
use super::utils;


#[derive(Clone, Debug)]
pub struct TodoState {
  is_done: bool,
  is_editing: bool,
  name: String,
  input: Option<HtmlInputElement>
}


pub struct Todo {
  pub gizmo: Gizmo,
  pub index: usize,
  pub state: Arc<Mutex<TodoState>>
}


impl Todo {
  pub fn new(
    index: usize,
    name: String,
    tx_msg: Transmitter<TodoListMsg>,
    rx_end_edit_externally: Receiver<()>
  ) -> Todo {
    trace!("Creating new todo item - {} {}", name, index);
    // Here's our big shared todo state.
    let state =
      Arc::new(
        Mutex::new(
          TodoState {
            is_done: false,
            is_editing: false,
            name: name.to_string(),
            input: None
          }
        )
      );
    let tx_stop_editing = rx_end_edit_externally.new_trns();
    let bldr = todo_item(index, state, tx_msg);
    let gizmo =
      bldr
      .build()
      .unwrap();
    let todo =
      Todo {
        index,
        gizmo,
        state
      };
    todo
  }
}

fn view(
  name: String,
  tx_toggle_completion_on_click: Transmitter<()>,
  tx_start_edit: Transmitter<()>,
  tx_remove_on_click: Transmitter<()>,
  rx_todo_state: Receiver<TodoState>,
  rx_todo_name: Receiver<String>,
  input_transmitters: (Transmitter<HtmlElement>, Transmitter<Event>, Transmitter<Event>)
) -> GizmoBuilder {
  let rx_class =
    rx_todo_state
    .branch_map(|state| {
      if state.is_editing {
        "editing"
      } else if state.is_done {
        "completed"
      } else {
        ""
      }.to_string()
    });

  let (tx_post_build, tx_on_blur, tx_on_keyup) = input_transmitters;

  li()
    .rx_class("", rx_class)
    .with(
      div()
        .class("view")
        .with(
          input()
            .class("toggle")
            .attribute("type", "checkbox")
            .style("cursor", "pointer")
            .tx_on_map("click", tx_toggle_completion_on_click, |_| ())
        )
        .with(
          label()
            .rx_text(&name, rx_todo_name)
            .tx_on_map("dblclick", tx_start_edit, |_| ())
        )
        .with(
          button()
            .class("destroy")
            .style("cursor", "pointer")
            .tx_on_map("click", tx_remove_on_click, |_| ())
        )
    )
    .with(
      input()
        .tx_post_build(tx_post_build)
        .class("edit")
        .value(&name)
        .tx_on("blur", tx_on_blur)
        .tx_on("keyup", tx_on_keyup)
    )
}


pub fn todo_item(
  index: usize,
  todo_state:Arc<Mutex<TodoState>>,
  tx_msg: Transmitter<TodoListMsg>,
) -> GizmoBuilder {
  // The name of this todo can be edited, so we'll need a tx rx for that.
  let (tx_todo_name, rx_todo_name) = txrx();

  // First get most of our wiring
  let ( tx_remove_on_click,
        tx_toggle_completion_on_click,
        tx_start_edit,
        rx_edited_name,
        rx_todo_state
  ) =
  {
    // We will have a button to click to remove the todo. Forward clicks from the
    // button to transmit messages into tx_msg. We're left with a tx to send clicks
    // into.
    let tx_remove_on_click = {
      let (tx_remove, rx_remove) = txrx();
      rx_remove.forward_map(
        &tx_msg,
        move |_| TodoListMsg::TodoRemove(index)
      );
      tx_remove
    };

    // We will have a toggle input to toggle completion of the todo on and off.
    // We're left with a tx to send clicks into and an rx that receives whether
    // or not the todo is marked complete.
    let (tx_toggle_completion_on_click, rx_is_complete) = {
      let mut tx_toggle = Transmitter::new();
      let rx_is_complete = Receiver::<bool>::new();
      let tx_msg_complete = tx_msg.clone();
      tx_toggle.wire_fold_shared(
        &rx_is_complete,
        todo_state.clone(),
        |todo, _| {
          todo.is_done = !todo.is_done;
          todo.is_done
        }
      );
      (tx_toggle, rx_is_complete)
    };

    // And we'll need some tx rx pairs for editing
    let (tx_start_edit, rx_start_edit) = txrx();
    //let (tx_end_edit_internally, rx_end_edit_internally) = txrx();
    let rx_edited_name = recv();

    // Use the editing and complete/uncomplete signals to updated state of the
    // todo.
    let (tx_todo_state, rx_todo_state) = txrx();
    {
      let tx = tx_todo_state.clone();
      rx_is_complete
        .branch()
        .respond_shared(
          todo_state.clone(),
          move |todo: &mut TodoState, is_complete| {
            todo.is_done = *is_complete;
            tx.send(&todo.clone());
          }
        );
    }

    {
      let tx = tx_todo_state.clone();
      rx_start_edit
        .respond_shared(
          todo_state.clone(),
          move |todo, _| {
            todo.is_editing = true;
            tx.send(&todo.clone());

            // Focus the element - for some reason we have to do this async or else
            // its effectiveness is spotty
            let input:HtmlInputElement =
              todo
              .input
              .as_ref()
              .unwrap()
              .clone();
            timeout(0, move || {
              input
                .focus()
                .unwrap();
              false
            });
          }
        );
    }
    {
      let tx = tx_todo_state.clone();
      rx_edited_name
        .branch()
        .respond_shared(
          todo_state.clone(),
          move |todo:&mut TodoState, name:&String| {
            todo.is_editing = false;
            todo.name = name.to_string();
            tx.send(&todo.clone());
            tx_todo_name.send(name);
            tx_msg.send(&TodoListMsg::TodoUpdateName(index, name.to_string()));
          }
        );
    }

    ( tx_remove_on_click,
      tx_toggle_completion_on_click,
      tx_start_edit,
      rx_edited_name,
      rx_todo_state
    )
  };

  // There is an edit input that has some special transmitters we need to build
  let input_transmitters =
  {
    let tx_edited_name = rx_edited_name.new_trns();

    // Once the input is built we'll store the input element so we can use it in
    // our folding functions.
    let (tx_post_build, rx_post_build) = txrx();
    rx_post_build.respond_shared(
      todo_state.clone(),
      |todo: &mut TodoState, el:&HtmlElement| {
        let input:HtmlInputElement =
          el
          .clone()
          .dyn_into::<HtmlInputElement>()
          .unwrap();
        todo.input = Some(input);
      }
    );

    // On blur we're going to update the name of the todo
    let (tx_on_blur, rx_on_blur) = txrx();
    rx_on_blur.forward_filter_fold_shared(
      &tx_edited_name,
      todo_state.clone(),
      |todo, _:&Event| {
        trace!("blur");
        utils::input_value(todo.input.as_ref().unwrap())
      }
    );

    // On keyup we're going to check some things and then possibly update the
    // name of the todo
    let (tx_on_keyup, rx_on_keyup) = txrx();
    rx_on_keyup.forward_filter_fold_shared(
      &tx_edited_name,
      todo_state.clone(),
      |todo, ev:&Event| {
        trace!("keyup");
        let kev =
          ev
          .dyn_ref::<KeyboardEvent>()
          .unwrap();
        let key =
          kev.key();
        let input = todo.input.as_ref().unwrap();
        if key == "Enter" {
          utils::input_value(input)
        } else if key == "Escape" {
          input.set_value(&todo.name);
          Some(todo.name.clone())
        } else {
          None
        }
      }
    );

    (tx_post_build, tx_on_blur, tx_on_keyup)
  };

  view(
    name,
    tx_toggle_completion_on_click,
    tx_start_edit,
    tx_remove_on_click,
    rx_todo_state,
    rx_todo_name,
    input_transmitters
  )
}
