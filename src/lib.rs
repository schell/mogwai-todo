#[macro_use]
extern crate log;
extern crate console_log;
extern crate console_error_panic_hook;
extern crate mogwai;

mod utils;

mod todo;

mod todo_list;
use todo_list::*;

use log::Level;
use mogwai::prelude::*;
use wasm_bindgen::prelude::*;


// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;


/// Creates the input element into which the user adds new todos. Also returns
/// a receiver that receives new todos.
fn mk_todo_input() -> (GizmoBuilder, Receiver<String>) {
  // tx_todo_input_event will transmit change events from our todo input
  let mut tx_todo_input_event:Transmitter<Event> =
    Transmitter::new();
  // rx_todo_input will receive the text value of our new todos
  let rx_todo_input:Receiver<String> =
    Receiver::new();
  // Wire transmission of input events to the receive todo names as strings.
  // Make sure this only happens when the string is not empty.
  tx_todo_input_event.wire_filter_map(
    &rx_todo_input,
    |ev:&Event| -> Option<String> {
      let todo_name = utils::event_input_value(ev).unwrap();
      if todo_name.is_empty() {
        None
      } else {
        trace!("Got todo input: {:?}", todo_name);
        Some(todo_name)
      }
    }
  );

  // Whenever a new todo comes down the pipe, clear out the input text
  // Branch the receiver so we can forward without mutating and overwriting the
  // original.
  let rx_todo_input_value:Receiver<String> =
    rx_todo_input.branch_map(|_| {
      trace!("Clearing out the input value");
      "".into()
    });

  let input =
    input()
    .class("new-todo")
    .attribute("placeholder", "What needs to be done?")
    .boolean_attribute("autofocus")
    .tx_on("change", tx_todo_input_event)
    .rx_value("", rx_todo_input_value);

  (input, rx_todo_input)
}


/// This is the inner footer that hold some controls
fn todo_footer(
  mut tx_display: Transmitter<bool>,
  mut tx_num_items: Transmitter<usize>
) -> GizmoBuilder {
  // Wire the display tx to a style rx to hide the footer
  let rx_display = Receiver::<String>::new();
  tx_display.wire_map(&rx_display, todo_list::mk_bool_to_display());

  // Wire the number of items to text
  let rx_num_items = Receiver::<String>::new();
  tx_num_items.wire_map(&rx_num_items, |n| {
    let items =
      if *n == 1 {
        "item"
      } else {
        "items"
      };
    format!("{} {} left", n, items)
  });

  footer()
    .class("footer")
    .rx_style("display", "none", rx_display)
    .with(
      span()
        .class("todo-count")
        .with(
          strong()
            .rx_text("0", rx_num_items)
        )
    )
    .with(
      ul()
        .class("filters")
        .with(
          li()
            .with(
              a()
                .class("selected")
                .attribute("href", "#/")
                .text("All")
            )
        )
        .with(
          li()
            .with(
              a()
                .attribute("href", "#/active")
                .text("Active")
            )
        )
        .with(
          li()
            .with(
              a()
                .attribute("href", "#/completed")
                .text("Completed")
            )
        )
    )
}



#[wasm_bindgen]
pub fn main() -> Result<(), JsValue> {
  utils::set_panic_hook();

  console_log::init_with_level(Level::Trace)
    .unwrap();

  trace!("Hello from mogwai-todo");

  // Create our main input to create new todos, and the receiver that gets them
  let (todo_input, rx_todo) = mk_todo_input();
  // Create a transmitter to tell when to display the todo section and footer
  let tx_display = Transmitter::new();
  // Create a transmitter for the number of todo items left
  let tx_num_items = Transmitter::new();

  let todo_main_section =
    todo_main_section(tx_display.clone(), rx_todo, tx_num_items.clone());
  let todo_footer =
    todo_footer(tx_display, tx_num_items);

  section()
    .class("todoapp")
    .with(
      header()
        .class("header")
        .with(
          h1()
            .text("todos")
        )
        .with(todo_input)
    )
    .with(
      todo_main_section
    )
    .with(
      todo_footer
    )
    .build()?
    .run()?;

  footer()
    .class("info")
    .with(
      p()
        .text("Double click to edit a todo")
    )
    .with(
      p()
        .text("Written by ")
        .with(
          a()
            .attribute("href", "https://github.com/schell")
            .text("Schell Scivally")
        )
    )
    .with(
      p()
        .text("Part of ")
        .with(
          a()
            .attribute("href", "http://todomvc.com")
            .text("TodoMVC")
        )
    )
    .build()?
    .run()
}
