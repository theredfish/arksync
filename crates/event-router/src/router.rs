// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Synchronous, in-process event routing.

use crate::EventEnvelope;
use alloc::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventHandlerError {
    Rejected,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DispatchReport {
    pub matched: usize,
    pub delivered: usize,
    pub rejected: usize,
}

/// Decides whether a subscription should receive an event.
pub trait EventFilter<Payload, Source = ()> {
    fn matches(&self, event: &EventEnvelope<Payload, Source>) -> bool;
}

impl<Payload, Source, Filter> EventFilter<Payload, Source> for Filter
where
    Filter: Fn(&EventEnvelope<Payload, Source>) -> bool,
{
    fn matches(&self, event: &EventEnvelope<Payload, Source>) -> bool {
        self(event)
    }
}

impl<Payload, Source> EventFilter<Payload, Source> for () {
    fn matches(&self, _event: &EventEnvelope<Payload, Source>) -> bool {
        true
    }
}

/// Handles one event selected by a local subscription.
pub trait EventHandler<Payload, Source = ()> {
    fn handle(&mut self, event: &EventEnvelope<Payload, Source>) -> Result<(), EventHandlerError>;
}

impl<Payload, Source, Handler> EventHandler<Payload, Source> for Handler
where
    Handler: FnMut(&EventEnvelope<Payload, Source>) -> Result<(), EventHandlerError>,
{
    fn handle(&mut self, event: &EventEnvelope<Payload, Source>) -> Result<(), EventHandlerError> {
        self(event)
    }
}

pub struct EventSubscription<Payload, Source = ()> {
    filter: Box<dyn EventFilter<Payload, Source> + Send>,
    handler: Box<dyn EventHandler<Payload, Source> + Send>,
}

impl<Payload, Source> EventSubscription<Payload, Source> {
    pub fn new<Filter, Handler>(filter: Filter, handler: Handler) -> Self
    where
        Filter: EventFilter<Payload, Source> + Send + 'static,
        Handler: EventHandler<Payload, Source> + Send + 'static,
    {
        Self {
            filter: Box::new(filter),
            handler: Box::new(handler),
        }
    }

    fn matches(&self, event: &EventEnvelope<Payload, Source>) -> bool {
        self.filter.matches(event)
    }

    fn handle(&mut self, event: &EventEnvelope<Payload, Source>) -> Result<(), EventHandlerError> {
        self.handler.handle(event)
    }
}

/// Runtime-independent router for synchronous, in-process fan-out.
pub struct EventRouter<Payload, Source = ()> {
    subscriptions: Vec<EventSubscription<Payload, Source>>,
}

impl<Payload, Source> Default for EventRouter<Payload, Source> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Payload, Source> EventRouter<Payload, Source> {
    pub fn new() -> Self {
        Self {
            subscriptions: Vec::new(),
        }
    }

    pub fn add_subscription(&mut self, subscription: EventSubscription<Payload, Source>) {
        self.subscriptions.push(subscription);
    }

    pub fn subscribe<Handler>(&mut self, handler: Handler)
    where
        Handler: EventHandler<Payload, Source> + Send + 'static,
    {
        self.subscribe_where((), handler);
    }

    pub fn subscribe_where<Filter, Handler>(&mut self, filter: Filter, handler: Handler)
    where
        Filter: EventFilter<Payload, Source> + Send + 'static,
        Handler: EventHandler<Payload, Source> + Send + 'static,
    {
        self.add_subscription(EventSubscription::new(filter, handler));
    }

    pub fn publisher(&mut self) -> EventPublisher<'_, Payload, Source> {
        EventPublisher { router: self }
    }

    pub fn publish(&mut self, event: &EventEnvelope<Payload, Source>) -> DispatchReport {
        let mut report = DispatchReport::default();

        for subscription in &mut self.subscriptions {
            if !subscription.matches(event) {
                continue;
            }

            report.matched += 1;
            match subscription.handle(event) {
                Ok(()) => report.delivered += 1,
                Err(EventHandlerError::Rejected) => report.rejected += 1,
            }
        }

        report
    }
}

/// Borrowed publishing handle for a local [`EventRouter`].
pub struct EventPublisher<'router, Payload, Source = ()> {
    router: &'router mut EventRouter<Payload, Source>,
}

impl<Payload, Source> EventPublisher<'_, Payload, Source> {
    pub fn publish(&mut self, event: EventEnvelope<Payload, Source>) -> DispatchReport {
        self.router.publish(&event)
    }
}
