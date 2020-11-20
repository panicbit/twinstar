//! Utilities for routing requests
//!
//! See [`RoutingNode`] for details on how routes are matched.

use uriparse::path::Path;

use std::collections::HashMap;
use std::convert::TryInto;

use crate::Handler;
use crate::types::Request;

#[derive(Default)]
/// A node for routing requests
///
/// Routing is processed by a tree, with each child being a single path segment.  For
/// example, if a handler existed at "/trans/rights", then the root-level node would have
/// a child "trans", which would have a child "rights".  "rights" would have no children,
/// but would have an attached handler.
///
/// If one route is shorter than another, say "/trans/rights" and
/// "/trans/rights/r/human", then the longer route always matches first, so a request for
/// "/trans/rights/r/human/rights" would be routed to "/trans/rights/r/human", and
/// "/trans/rights/now" would route to "/trans/rights"
///
/// Routing is only performed on normalized paths, so "/endpoint" and "/endpoint/" are
/// considered to be the same route.
pub struct RoutingNode(Option<Handler>, HashMap<String, Self>);

impl RoutingNode {
    /// Attempt to identify a handler based on path segments
    ///
    /// This searches the network of routing nodes attempting to match a specific request,
    /// represented as a sequence of path segments.  For example, "/dir/image.png?text"
    /// should be represented as `&["dir", "image.png"]`.
    ///
    /// See [`RoutingNode`] for details on how routes are matched.
    pub fn match_path<I,S>(&self, path: I) -> Option<&Handler>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<str>,
    {
        let mut node = self;
        let mut path = path.into_iter();
        let mut last_seen_handler = None;
        loop {
            let Self(maybe_handler, map) = node;

            last_seen_handler = maybe_handler.as_ref().or(last_seen_handler);

            if let Some(segment) = path.next() {
                if let Some(route) = map.get(segment.as_ref()) {
                    node = route;
                } else {
                    return last_seen_handler;
                }
            } else {
                return last_seen_handler;
            }
        }
    }

    /// Attempt to identify a route for a given [`Request`]
    ///
    /// See [`RoutingNode`] for details on how routes are matched.
    pub fn match_request(&self, req: Request) -> Option<&Handler> {
        let mut path = req.path().to_owned();
        path.normalize(false);
        self.match_path(path.segments())
    }

    /// Add a route to the network
    ///
    /// This method wraps [`add_route_by_path()`](Self::add_route_by_path()) while
    /// unwrapping any errors that might occur.  For this reason, this method only takes
    /// static strings.  If you would like to add a string dynamically, please use
    /// [`RoutingNode::add_route_by_path()`] in order to appropriately deal with any
    /// errors that might arise.
    pub fn add_route(&mut self, path: &'static str, handler: impl Into<Handler>) {
        let path: Path = path.try_into().expect("Malformed path route received");
        self.add_route_by_path(path, handler).unwrap();
    }

    /// Add a route to the network
    ///
    /// The path provided MUST be absolute.  Callers should verify this before calling
    /// this method.
    ///
    /// For information about how routes work, see [`RoutingNode::match_path()`]
    pub fn add_route_by_path(&mut self, mut path: Path, handler: impl Into<Handler>) -> Result<(), ConflictingRouteError>{
        debug_assert!(path.is_absolute());
        path.normalize(false);

        let mut node = self;
        for segment in path.segments() {
            node = node.1.entry(segment.to_string()).or_default();
        }

        if node.0.is_some() {
            Err(ConflictingRouteError())
        } else {
            node.0 = Some(handler.into());
            Ok(())
        }
    }

    /// Recursively shrink maps to fit
    pub fn shrink(&mut self) {
        let mut to_shrink = vec![&mut self.1];
        while let Some(shrink) = to_shrink.pop() {
            shrink.shrink_to_fit();
            to_shrink.extend(shrink.values_mut().map(|n| &mut n.1));
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConflictingRouteError();

impl std::error::Error for ConflictingRouteError { }

impl std::fmt::Display for ConflictingRouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Attempted to create a route with the same matcher as an existing route")
    }
}
