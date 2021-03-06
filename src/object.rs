//! Items in the scene heirarchy.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::mpsc;

use mint;

use hub::{Hub, Message, Operation, SubNode};
use node::NodePointer;


//Note: no local state should be here, only remote links
/// `Base` represents a concrete entity that can be added to the scene.
///
/// One cannot construct `Base` directly. Wrapper types such as [`Camera`],
/// [`Mesh`], and [`Group`] are provided instead.
///
/// Any type that implements [`Object`] may be converted to its concrete
/// `Base` type with the method [`Object::upcast`]. This is useful for
/// storing generic objects in containers.
///
/// [`Camera`]: ../camera/struct.Camera.html
/// [`Mesh`]: ../mesh/struct.Mesh.html
/// [`Group`]: ../object/struct.Group.html
/// [`Object`]: ../object/trait.Object.html
/// [`Object::upcast`]: ../object/trait.Object.html#method.upcast
#[derive(Clone)]
pub struct Base {
    pub(crate) node: NodePointer,
    pub(crate) tx: mpsc::Sender<Message>,
}

/// Marks data structures that are able to added to the scene graph.
pub trait Object: AsRef<Base> + AsMut<Base> {
    /// Converts into the base type.
    fn upcast(&self) -> Base {
        self.as_ref().clone()
    }

    /// Invisible objects are not rendered by cameras.
    fn set_visible(
        &self,
        visible: bool,
    ) {
        self.as_ref().set_visible(visible)
    }

    /// Rotates object in the specific direction of `target`.
    fn look_at<E, T>(
        &self,
        eye: E,
        target: T,
        up: Option<mint::Vector3<f32>>,
    ) where
        Self: Sized,
        E: Into<mint::Point3<f32>>,
        T: Into<mint::Point3<f32>>,
    {
        self.as_ref().look_at(eye, target, up)
    }

    /// Set both position, orientation and scale.
    fn set_transform<P, Q>(
        &self,
        pos: P,
        rot: Q,
        scale: f32,
    ) where
        Self: Sized,
        P: Into<mint::Point3<f32>>,
        Q: Into<mint::Quaternion<f32>>,
    {
        self.as_ref().set_transform(pos, rot, scale)
    }

    /// Set position.
    fn set_position<P>(
        &self,
        pos: P,
    ) where
        Self: Sized,
        P: Into<mint::Point3<f32>>,
    {
        self.as_ref().set_position(pos)
    }

    /// Set orientation.
    fn set_orientation<Q>(
        &self,
        rot: Q,
    ) where
        Self: Sized,
        Q: Into<mint::Quaternion<f32>>,
    {
        self.as_ref().set_orientation(rot)
    }

    /// Set scale.
    fn set_scale(
        &self,
        scale: f32,
    ) {
        self.as_ref().set_scale(scale)
    }
}

impl PartialEq for Base {
    fn eq(
        &self,
        other: &Base,
    ) -> bool {
        self.node == other.node
    }
}

impl Eq for Base {}

impl Hash for Base {
    fn hash<H: Hasher>(
        &self,
        state: &mut H,
    ) {
        self.node.hash(state);
    }
}

impl fmt::Debug for Base {
    fn fmt(
        &self,
        f: &mut fmt::Formatter,
    ) -> fmt::Result {
        self.node.fmt(f)
    }
}

impl Base {
    /// Invisible objects are not rendered by cameras.
    pub fn set_visible(
        &self,
        visible: bool,
    ) {
        let msg = Operation::SetVisible(visible);
        let _ = self.tx.send((self.node.downgrade(), msg));
    }

    /// Rotates object in the specific direction of `target`.
    pub fn look_at<E, T>(
        &self,
        eye: E,
        target: T,
        up: Option<mint::Vector3<f32>>,
    ) where
        E: Into<mint::Point3<f32>>,
        T: Into<mint::Point3<f32>>,
    {
        use cgmath::{InnerSpace, Point3, Quaternion, Rotation, Vector3};
        let p: [mint::Point3<f32>; 2] = [eye.into(), target.into()];
        let dir = (Point3::from(p[0]) - Point3::from(p[1])).normalize();
        let z = Vector3::unit_z();
        let up = match up {
            Some(v) => Vector3::from(v).normalize(),
            None if dir.dot(z).abs() < 0.99 => z,
            None => Vector3::unit_y(),
        };
        let q = Quaternion::look_at(dir, up).invert();
        self.set_transform(p[0], q, 1.0);
    }

    /// Set both position, orientation and scale.
    pub fn set_transform<P, Q>(
        &self,
        pos: P,
        rot: Q,
        scale: f32,
    ) where
        P: Into<mint::Point3<f32>>,
        Q: Into<mint::Quaternion<f32>>,
    {
        let msg = Operation::SetTransform(Some(pos.into()), Some(rot.into()), Some(scale));
        let _ = self.tx.send((self.node.downgrade(), msg));
    }

    /// Set position.
    pub fn set_position<P>(
        &self,
        pos: P,
    ) where
        P: Into<mint::Point3<f32>>,
    {
        let msg = Operation::SetTransform(Some(pos.into()), None, None);
        let _ = self.tx.send((self.node.downgrade(), msg));
    }

    /// Set orientation.
    pub fn set_orientation<Q>(
        &self,
        rot: Q,
    ) where
        Q: Into<mint::Quaternion<f32>>,
    {
        let msg = Operation::SetTransform(None, Some(rot.into()), None);
        let _ = self.tx.send((self.node.downgrade(), msg));
    }

    /// Set scale.
    pub fn set_scale(
        &self,
        scale: f32,
    ) {
        let msg = Operation::SetTransform(None, None, Some(scale));
        let _ = self.tx.send((self.node.downgrade(), msg));
    }
}

impl AsRef<Base> for Base {
    fn as_ref(&self) -> &Base {
        self
    }
}

/// Groups are used to combine several other objects or groups to work with them
/// as with a single entity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Group {
    object: Base,
}
three_object!(Group::object);

impl Group {
    pub(crate) fn new(hub: &mut Hub) -> Self {
        let sub = SubNode::Group { first_child: None };
        Group {
            object: hub.spawn(sub),
        }
    }

    /// Add new [`Base`](struct.Base.html) to the group.
    pub fn add<P>(
        &self,
        child: P,
    ) where
        P: AsRef<Base>,
    {
        let msg = Operation::AddChild(child.as_ref().node.clone());
        let _ = self.object.tx.send((self.object.node.downgrade(), msg));
    }

    /// Removes a child [`Base`](struct.Base.html) from the group.
    pub fn remove<P>(
        &self,
        child: P,
    ) where
        P: AsRef<Base>,
    {
        let msg = Operation::RemoveChild(child.as_ref().node.clone());
        let _ = self.object.tx.send((self.object.node.downgrade(), msg));
    }
}
