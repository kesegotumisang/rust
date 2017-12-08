// Copyright 2017 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(warnings)]

use std::mem;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use rustc_data_structures::sync::{Lock, LockGuard, Lrc};
use rayon_core::registry::{self, Registry};
use rayon_core::fiber::WaiterLatch;
use rayon_core::latch::Latch;
use syntax_pos::Span;
use ty::maps::Query;
use errors::Diagnostic;

pub struct QueryJob<'tcx> {
    pub latch: WaiterLatch,
    pub stack: Vec<(Span, Query<'tcx>)>,
    pub track_diagnostics: bool,
    pub tls: bool,
    pub diagnostics: Lock<Vec<Diagnostic>>,
}

impl<'tcx> QueryJob<'tcx> {
    pub fn new(stack: Vec<(Span, Query<'tcx>)>, track_diagnostics: bool, tls: bool) -> Self {
        QueryJob {
            latch: WaiterLatch::new(),
            track_diagnostics,
            diagnostics: Lock::new(Vec::new()),
            stack,
            tls,
        }
    }

    pub fn start(&self) {
        self.latch.start()
    }

    pub fn await(&self) {
        #[cfg(parallel_queries)]
        registry::in_worker(|worker, _| {
            unsafe {
                worker.wait_until(&self.latch);
            }
        });
    }

    pub fn signal_complete(&self) {
        #[cfg(parallel_queries)]
        {
            self.latch.set();
            Registry::current().signal();
        }
    }
}

pub(super) enum QueryResult<'tcx, T> {
    Started(Lrc<QueryJob<'tcx>>),
    Complete(T),
}
