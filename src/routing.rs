//! Utilities for routing requests
//!
//! See [`RoutingNode`] for details on how routes are matched.

use uriparse::path::{Path, Segment};

use std::collections::HashMap;
use std::convert::TryInto;

use crate::types::Request;

/// A node for linking values to routes
///
/// Routing is processed by a tree, with each child being a single path segment.  For
/// example, if an entry existed at "/trans/rights", then the root-level node would have
/// a child "trans", which would have a child "rights".  "rights" would have no children,
/// but would have an attached entry.
///
/// If one route is shorter than another, say "/trans/rights" and
/// "/trans/rights/r/human", then the longer route always matches first, so a request for
/// "/trans/rights/r/human/rights" would be routed to "/trans/rights/r/human", and
/// "/trans/rights/now" would route to "/trans/rights"
///
/// Routing is only performed on normalized paths, so "/endpoint" and "/endpoint/" are
/// considered to be the same route.
pub struct RoutingNode<T>(Option<T>, HashMap<String, Self>);

impl<T> RoutingNode<T> {
    /// Attempt to find and entry based on path segments
    ///
    /// This searches the network of routing nodes attempting to match a specific request,
    /// represented as a sequence of path segments.  For example, "/dir/image.png?text"
    /// should be represented as `&["dir", "image.png"]`.
    ///
    /// If a match is found, it is returned, along with the segments of the path trailing
    /// the subpath matcing the route.  For example, a route `/foo` recieving a request to
    /// `/foo/bar` would produce `vec!["bar"]`
    ///
    /// See [`RoutingNode`] for details on how routes are matched.
    pub fn match_path<I,S>(&self, path: I) -> Option<(Vec<S>, &T)>
    where
        I: IntoIterator<Item=S>,
        S: AsRef<str>,
    {
        let mut node = self;
        let mut path = path.into_iter().filter(|seg| !seg.as_ref().is_empty());
        let mut last_seen_handler = None;
        let mut since_last_handler = Vec::new();
        loop {
            let Self(maybe_handler, map) = node;

            if maybe_handler.is_some() {
                last_seen_handler = maybe_handler.as_ref();
                since_last_handler.clear();
            }

            if let Some(segment) = path.next() {
                let maybe_route = map.get(segment.as_ref());
                since_last_handler.push(segment);

                if let Some(route) = maybe_route {
                    node = route;
                } else {
                    break;
                }
            } else {
                break;
            }
        };

        if let Some(handler) = last_seen_handler {
            since_last_handler.extend(path);
            Some((since_last_handler, handler))
        } else {
            None
        }
    }

    /// Attempt to identify a route for a given [`Request`]
    ///
    /// See [`RoutingNode::match_path()`] for more information
    pub fn match_request(&self, req: &Request) -> Option<(Vec<String>, &T)> {
        let mut path = req.path().to_borrowed();
        path.normalize(false);
        self.match_path(path.segments())
            .map(|(segs, h)| (
                segs.into_iter()
                    .map(Segment::as_str)
                    .map(str::to_owned)
                    .collect(),
                h,
            ))
    }

    /// Add a route to the network
    ///
    /// This method wraps [`add_route_by_path()`](Self::add_route_by_path()) while
    /// unwrapping any errors that might occur.  For this reason, this method only takes
    /// static strings.  If you would like to add a string dynamically, please use
    /// [`RoutingNode::add_route_by_path()`] in order to appropriately deal with any
    /// errors that might arise.
    pub fn add_route(&mut self, path: &'static str, data: T) {
        let path: Path = path.try_into().expect("Malformed path route received");
        self.add_route_by_path(path, data).unwrap();
    }

    /// Add a route to the network
    ///
    /// The path provided MUST be absolute.  Callers should verify this before calling
    /// this method.
    ///
    /// For information about how routes work, see [`RoutingNode::match_path()`]
    pub fn add_route_by_path(&mut self, mut path: Path, data: T) -> Result<(), ConflictingRouteError>{
        debug_assert!(path.is_absolute());
        path.normalize(false);

        let mut node = self;
        for segment in path.segments() {
            if segment != "" {
                node = node.1.entry(segment.to_string()).or_default();
            }
        }

        if node.0.is_some() {
            Err(ConflictingRouteError())
        } else {
            node.0 = Some(data);
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

impl<T> Default for RoutingNode<T> {
    fn default() -> Self {
        Self(None, HashMap::default())
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
