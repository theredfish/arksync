// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! no_std EventBus core.
//!
//! The bus owns the generic subscription/filter/handler mechanics. Bounded
//! contexts own their event payloads and can attach any local, MQTT, or storage
//! handler later.

use crate::EventEnvelope;
use alloc::{boxed::Box, string::String, vec::Vec};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventBusError {
    HandlerRejected,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Delivery {
    Local,
    Mqtt { topic: String },
    Both { mqtt_topic: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Persistence {
    Memory,
    Storage,
}

/// Decides whether a subscription should receive an event.
pub trait EventFilter<Payload, Source = ()> {
    fn matches(&self, event: &EventEnvelope<Payload, Source>) -> bool;
}

impl<Payload, Source, Handler> EventFilter<Payload, Source> for Handler
where
    Handler: Fn(&EventEnvelope<Payload, Source>) -> bool,
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

/// Handles an event selected by a subscription.
///
/// A handler is what a subscription runs after its filter matched. It can
/// forward the event to a channel, write to storage, publish to MQTT, or run
/// local logic.
pub trait EventHandler<Payload, Source = ()> {
    fn handle(&mut self, event: EventEnvelope<Payload, Source>) -> Result<(), EventBusError>;
}

impl<Payload, Source, Handler> EventHandler<Payload, Source> for Handler
where
    Handler: FnMut(EventEnvelope<Payload, Source>) -> Result<(), EventBusError>,
{
    fn handle(&mut self, event: EventEnvelope<Payload, Source>) -> Result<(), EventBusError> {
        self(event)
    }
}

pub struct EventSubscription<Payload, Source = ()> {
    filter: Box<dyn EventFilter<Payload, Source>>,
    handler: Box<dyn EventHandler<Payload, Source>>,
    delivery: Delivery,
    persistence: Persistence,
}

impl<Payload, Source> EventSubscription<Payload, Source> {
    pub fn local<Filter, Handler>(filter: Filter, handler: Handler) -> Self
    where
        Filter: EventFilter<Payload, Source> + 'static,
        Handler: EventHandler<Payload, Source> + 'static,
    {
        Self {
            filter: Box::new(filter),
            handler: Box::new(handler),
            delivery: Delivery::Local,
            persistence: Persistence::Memory,
        }
    }

    pub fn delivery(&self) -> &Delivery {
        &self.delivery
    }

    pub fn persistence(&self) -> Persistence {
        self.persistence
    }

    fn matches(&self, event: &EventEnvelope<Payload, Source>) -> bool {
        self.filter.matches(event)
    }

    fn handle(&mut self, event: EventEnvelope<Payload, Source>) -> Result<(), EventBusError> {
        self.handler.handle(event)
    }
}

pub struct EventBus<Payload, Source = ()> {
    subscriptions: Vec<EventSubscription<Payload, Source>>,
}

impl<Payload, Source> Default for EventBus<Payload, Source> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Payload, Source> EventBus<Payload, Source> {
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
        Handler: EventHandler<Payload, Source> + 'static,
    {
        self.subscribe_where((), handler);
    }

    pub fn subscribe_where<Filter, Handler>(&mut self, filter: Filter, handler: Handler)
    where
        Filter: EventFilter<Payload, Source> + 'static,
        Handler: EventHandler<Payload, Source> + 'static,
    {
        self.add_subscription(EventSubscription::local(filter, handler));
    }

    pub fn producer(&mut self) -> EventProducer<'_, Payload, Source> {
        EventProducer { bus: self }
    }
}

impl<Payload, Source> EventBus<Payload, Source>
where
    Payload: Clone,
    Source: Clone,
{
    pub fn publish(
        &mut self,
        event: EventEnvelope<Payload, Source>,
    ) -> Result<usize, EventBusError> {
        let mut delivered = 0;

        for subscription in &mut self.subscriptions {
            if !subscription.matches(&event) {
                continue;
            }

            subscription.handle(event.clone())?;
            delivered += 1;
        }

        Ok(delivered)
    }
}

pub struct EventProducer<'bus, Payload, Source = ()> {
    bus: &'bus mut EventBus<Payload, Source>,
}

impl<Payload, Source> EventProducer<'_, Payload, Source>
where
    Payload: Clone,
    Source: Clone,
{
    pub fn publish(
        &mut self,
        event: EventEnvelope<Payload, Source>,
    ) -> Result<usize, EventBusError> {
        self.bus.publish(event)
    }
}
