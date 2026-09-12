use std::{io, pin::Pin, time::Instant};

use crossterm::event::{Event, EventStream};
use futures_util::{Stream, StreamExt};
use tokio::{sync::mpsc, time::sleep_until};

use super::UiEvent;

/// Tokio-compatible source that merges terminal input, application messages,
/// and an optional redraw deadline without coupling Ratatool to application tasks.
pub struct AsyncEventSource<M> {
    terminal: Pin<Box<dyn Stream<Item = io::Result<Event>> + Send>>,
    messages: mpsc::Receiver<M>,
    messages_open: bool,
}

impl<M> AsyncEventSource<M> {
    pub fn channel(capacity: usize) -> (mpsc::Sender<M>, Self) {
        let (sender, messages) = mpsc::channel(capacity);
        (
            sender,
            Self {
                terminal: Box::pin(EventStream::new()),
                messages,
                messages_open: true,
            },
        )
    }

    pub async fn next(&mut self, deadline: Option<Instant>) -> io::Result<Option<UiEvent<M>>> {
        loop {
            let event = if let Some(deadline) = deadline {
                tokio::select! {
                    message = self.messages.recv(), if self.messages_open => {
                        match message {
                            Some(message) => return Ok(Some(UiEvent::Message(message))),
                            None => {
                                self.messages_open = false;
                                continue;
                            }
                        }
                    }
                    () = sleep_until(deadline.into()) => return Ok(Some(UiEvent::Tick)),
                    event = self.terminal.next() => event,
                }
            } else {
                tokio::select! {
                    message = self.messages.recv(), if self.messages_open => {
                        match message {
                            Some(message) => return Ok(Some(UiEvent::Message(message))),
                            None => {
                                self.messages_open = false;
                                continue;
                            }
                        }
                    }
                    event = self.terminal.next() => event,
                }
            };

            let Some(event) = event else {
                return Ok(None);
            };
            if let Some(event) = UiEvent::from_terminal(event?) {
                return Ok(Some(event));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::task::Poll;

    use futures_util::stream;

    use super::*;

    #[tokio::test]
    async fn closed_message_channel_does_not_close_terminal_event_source() {
        let (sender, messages) = mpsc::channel(1);
        let mut first_poll = true;
        let mut source = AsyncEventSource::<()> {
            terminal: Box::pin(stream::poll_fn(move |context| {
                if first_poll {
                    first_poll = false;
                    context.waker().wake_by_ref();
                    Poll::Pending
                } else {
                    Poll::Ready(Some(Ok(Event::Resize(80, 24))))
                }
            })),
            messages,
            messages_open: true,
        };
        drop(sender);

        let event = source.next(None).await.unwrap();

        assert_eq!(
            event,
            Some(UiEvent::Resize {
                width: 80,
                height: 24,
            })
        );
        assert!(!source.messages_open);
    }
}
