// TODO add tests
use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::ops::{Deref, DerefMut};
use std::rc::{Rc, Weak};

use crate::{Commitment, Slot};

type WeakNode = Weak<SlotNode>;

// TODO add support for tracking the length
pub struct SlotTree {
    root: StrongNode,
    lookup: HashMap<Slot, WeakNode>,
    bootstrapping: bool,
    //len: usize,
}

#[derive(Clone, Copy)]
pub enum SlotStatus {
    Processed = 1,
    Confirmed = 2,
    Rooted = 3,
}

#[derive(Default)]
struct SlotNode {
    parent: RefCell<WeakNode>,
    children: RefCell<HashMap<Slot, StrongNode>>,
    slot: Slot,
    status: RefCell<SlotStatus>,
}

#[derive(Clone, Default)]
struct StrongNode(Rc<SlotNode>);

impl Deref for StrongNode {
    type Target = SlotNode;

    fn deref(&self) -> &Self::Target {
        self.0.deref()
    }
}

impl StrongNode {
    fn new(node: SlotNode) -> Self {
        let rc = Rc::new(node);
        Self(rc)
    }

    #[inline]
    fn downgrade(&self) -> WeakNode {
        Rc::downgrade(&self.0)
    }

    #[inline]
    fn prune(self) -> Vec<Slot> {
        let mut pruned = vec![self.slot];
        let mut orphans = Vec::new();
        orphans.extend(self.children.take().into_values());
        while let Some(node) = orphans.pop() {
            pruned.push(node.slot);
            let children = mem::take(node.children.borrow_mut().deref_mut());
            orphans.extend(children.into_values());
        }
        pruned
    }
}

impl PartialEq for StrongNode {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl PartialEq for SlotNode {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot
    }
}

pub trait RawSlot {
    fn slot(&self) -> Slot;
    fn parent(&self) -> Slot;
    fn status(&self) -> SlotStatus;
}

impl SlotNode {
    fn new(slot: Slot, status: SlotStatus) -> Self {
        let children = RefCell::default();
        let status = RefCell::new(status);
        let parent = RefCell::default();

        Self {
            slot,
            parent,
            children,
            status,
        }
    }

    #[inline]
    fn attach(&self, child: StrongNode) {
        self.children.borrow_mut().insert(child.slot, child);
    }

    #[inline]
    fn detach(&self, child: Slot) -> Option<StrongNode> {
        self.children.borrow_mut().remove(&child)
    }

    #[inline]
    pub fn parent(&self) -> Option<Rc<SlotNode>> {
        self.parent.borrow().upgrade()
    }

    #[inline]
    fn set_parent(&self, parent: WeakNode) {
        self.parent.replace(parent);
    }

    #[inline]
    fn set_status(&self, status: SlotStatus) {
        self.status.replace(status);
    }

    #[inline]
    fn rooted(&self) -> bool {
        matches!(*self.status.borrow(), SlotStatus::Rooted)
    }
}

pub struct RootedOrPrunedSlot {
    slot: Slot,
    rooted: bool,
}

impl Deref for RootedOrPrunedSlot {
    type Target = Slot;

    fn deref(&self) -> &Self::Target {
        &self.slot
    }
}

impl RootedOrPrunedSlot {
    #[inline]
    fn new(slot: Slot) -> Self {
        Self {
            slot,
            rooted: false,
        }
    }
    #[inline]
    fn root(mut self) -> Self {
        self.rooted = true;
        self
    }

    #[inline]
    pub fn rooted(&self) -> bool {
        self.rooted
    }
}

impl From<RootedOrPrunedSlot> for Slot {
    fn from(rops: RootedOrPrunedSlot) -> Self {
        rops.slot
    }
}

impl SlotTree {
    pub fn new() -> Self {
        let root = StrongNode::new(SlotNode::default());
        let mut lookup = HashMap::default();
        let bootstrapping = true;
        lookup.insert(root.slot, StrongNode::downgrade(&root));

        Self {
            root,
            lookup,
            bootstrapping,
        }
    }

    fn bootstrap<T: RawSlot>(&mut self, raw: T) -> Option<Vec<RootedOrPrunedSlot>> {
        let parent = if let Some(parent) = self.lookup.get(&raw.parent()) {
            let parent = parent
                .upgrade()
                .expect("weak references in lookup table should always upgrade");
            parent.detach(raw.slot());
            parent
        } else {
            Rc::clone(&self.root.0)
        };
        let slot = if let Some(slot) = self.lookup.get(&raw.slot()) {
            let slot = slot
                .upgrade()
                .expect("weak references in lookup table should always upgrade");
            slot.set_status(raw.status());
            StrongNode(slot)
        } else {
            StrongNode::from(raw)
        };
        slot.set_parent(Rc::downgrade(&parent));
        let clone = StrongNode::clone(&slot);
        parent.attach(slot);
        if !clone.rooted() {
            return None;
        }

        let rooted_or_pruned = self.root(clone);
        self.bootstrapping = false;
        Some(rooted_or_pruned)
    }

    pub fn push<T: RawSlot>(&mut self, raw: T) -> Option<Vec<RootedOrPrunedSlot>> {
        if self.bootstrapping {
            return self.bootstrap(raw);
        }
        if raw.slot() <= self.root.slot {
            // shouldn't be able to modify already rooted nodes
            return None;
        }
        let weak = self.lookup.get(&raw.slot());

        let slot = if let Some(weak) = weak {
            // slot is already being tracked
            let slot = weak
                .upgrade()
                .expect("weak references in lookup table should always upgrade");
            let old_parent = slot
                .parent()
                .expect("nodes in lookup table should always have a parent");
            // try to find new parent of node in lookup table
            let new_parent = self
                .lookup
                .get(&raw.parent())?
                .upgrade()
                .expect("weak references in lookup table should always upgrade");

            // check whether slot has changed parents
            if old_parent != new_parent {
                // move the slot from old to new parent
                let slot = old_parent
                    .detach(slot.slot)
                    .expect("parents should always own their child slots");
                new_parent.attach(slot);
            }
            drop(old_parent);
            slot.set_parent(Rc::downgrade(&new_parent));
            slot.set_status(raw.status());
            StrongNode(slot)
        } else {
            // slot has never been seen before

            // check whether slot parent exists or else return root as parent
            let parent = self
                .lookup
                .get(&raw.parent())?
                .upgrade()
                .expect("weak references in lookup table should always upgrade");

            let slot = StrongNode::from(raw);
            slot.set_parent(Rc::downgrade(&parent));
            // create entry for further lookup operations
            self.lookup.insert(slot.slot, StrongNode::downgrade(&slot));
            let clone = StrongNode::clone(&slot);
            // move ownership of slot to its parent
            parent.attach(slot);
            clone
        };

        if !slot.rooted() {
            return None;
        }

        Some(self.root(slot))
    }

    fn root(&mut self, node: StrongNode) -> Vec<RootedOrPrunedSlot> {
        let mut parent = node
            .parent
            .borrow()
            .upgrade()
            .expect("active nodes should always have parents");
        let mut rooted_or_pruned = vec![RootedOrPrunedSlot::new(node.slot).root()];
        let mut child = node.slot;
        loop {
            let mut children = mem::take(parent.children.borrow_mut().deref_mut());
            // get the child of rooted branch
            children
                .remove(&child)
                .expect("rooted child should always be owned by rooted parent");
            // get all the child slots which were in rival branches and prune them
            for orphan in children.into_values() {
                let pruned = orphan.prune().into_iter().map(RootedOrPrunedSlot::new);
                rooted_or_pruned.extend(pruned);
            }
            if parent.rooted() {
                self.root = node;
                break;
            }
            rooted_or_pruned.push(RootedOrPrunedSlot::new(parent.slot).root());
            let temp = Weak::clone(&parent.parent.borrow());
            child = parent.slot;
            parent = temp
                .upgrade()
                .expect("active nodes should always have parents");
        }
        for s in &rooted_or_pruned {
            self.lookup.remove(s);
        }
        self.lookup
            .insert(self.root.slot, StrongNode::downgrade(&self.root));
        rooted_or_pruned
    }

    pub fn current_root(&self) -> Slot {
        self.root.slot
    }
}

impl Default for SlotStatus {
    fn default() -> Self {
        Self::Rooted
    }
}

impl<T: RawSlot> From<T> for SlotNode {
    fn from(raw: T) -> Self {
        let slot = raw.slot();
        let status = raw.status();
        Self::new(slot, status)
    }
}

impl<T: RawSlot> From<T> for StrongNode {
    fn from(raw: T) -> Self {
        Self::new(SlotNode::from(raw))
    }
}

impl From<Commitment> for SlotStatus {
    fn from(commitment: Commitment) -> Self {
        match commitment {
            Commitment::Processed => Self::Processed,
            Commitment::Confirmed => Self::Confirmed,
            Commitment::Finalized => Self::Rooted,
        }
    }
}

unsafe impl Send for SlotTree {}
