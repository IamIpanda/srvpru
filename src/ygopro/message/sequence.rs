// ============================================================
//  sequence
// ------------------------------------------------------------
/// Offer [struct_sequence!] for combining Structs.
// ============================================================

use serde::Serialize;
use serde::ser::SerializeTuple;
use crate::ygopro::message::Struct;
use crate::ygopro::message::MappedStruct;
use crate::ygopro::message::MessageType;

pub trait StructSequence : Struct {}

impl<T: StructSequence> MappedStruct for T {
    fn message() -> MessageType {
        MessageType::SRVPRU(crate::srvpru::message::MessageType::StructSequence)
    }
}

#[doc(hidden)]
macro_rules! count {
    () => (0usize);
    ( $x:tt $($xs:tt)* ) => (1usize + count!($($xs)*));
}

#[doc(hidden)]
macro_rules! define_struct_sequence {
    ($name: ident, $($T: ident, $n: tt),+) => {
        #[derive(Debug)]
        pub struct $name<$($T: Struct + MappedStruct + serde::Serialize),+>($(pub $T),+);
        impl<$($T),+> Serialize for $name<$($T),+> where $($T: Struct + MappedStruct + serde::Serialize),+ {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::ser::Serializer { 
                let mut data = Vec::new();
                $(data.append(&mut crate::ygopro::message::generate::wrap_mapped_struct(&self.$n));)*
                let mut seq = serializer.serialize_tuple(count!($($T)*))?;
                for byte in data.iter() {
                    seq.serialize_element(byte)?;
                }
                seq.end()
            }
        }
        impl<$($T),+> core::convert::From<($($T),+)> for $name<$($T),+> where $($T: Struct + MappedStruct + serde::Serialize),+ {
            fn from(source: ($($T),+)) -> Self {
                Self($(source.$n),+)
            }
        }
        impl<$($T),+> Struct for $name<$($T),+> where $($T: Struct + MappedStruct + serde::Serialize),+ {}
        impl<$($T),+> StructSequence for $name<$($T),+> where $($T: Struct + MappedStruct + serde::Serialize),+ {}
    };
}

define_struct_sequence!(StructSequence2, T1, 0, T2, 1);
define_struct_sequence!(StructSequence3, T1, 0, T2, 1, T3, 2);
define_struct_sequence!(StructSequence4, T1, 0, T2, 1, T3, 2, T4, 3);
define_struct_sequence!(StructSequence5, T1, 0, T2, 1, T3, 2, T4, 3, T5, 4);
define_struct_sequence!(StructSequence6, T1, 0, T2, 1, T3, 2, T4, 3, T5, 4, T6, 5);
define_struct_sequence!(StructSequence7, T1, 0, T2, 1, T3, 2, T4, 3, T5, 4, T6, 5, T7, 6);
define_struct_sequence!(StructSequence8, T1, 0, T2, 1, T3, 2, T4, 3, T5, 4, T6, 5, T7, 6, T8, 7);
define_struct_sequence!(StructSequence9, T1, 0, T2, 1, T3, 2, T4, 3, T5, 4, T6, 5, T7, 6, T8, 7, T9, 8);

#[doc(hidden)]
#[macro_export]
macro_rules! struct_sequence_name {
    ($s1:expr, $s2:expr) => (crate::ygopro::message::sequence::StructSequence2);
    ($s1:expr, $s2:expr, $s3:expr) => (crate::ygopro::message::sequence::StructSequence3);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr) => (crate::ygopro::message::sequence::StructSequence4);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr) => (crate::ygopro::message::sequence::StructSequence5);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr, $s6:expr) => (crate::ygopro::message::sequence::StructSequence6);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr, $s6:expr, $s7:expr) => (crate::ygopro::message::sequence::StructSequence7);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr, $s6:expr, $s7:expr, $s8:expr) => (crate::ygopro::message::sequence::StructSequence8);
    ($s1:expr, $s2:expr, $s3:expr, $s4:expr, $s5:expr, $s6:expr, $s7:expr, $s8:expr, $s9:expr) => (crate::ygopro::message::sequence::StructSequence9);
}

// ----------------------------------------------------------------------------------------------------
//  struct_sequence!
// ----------------------------------------------------------------------------------------------------
/// Generate a tuple, which keeps several messages, which still can normally serialize.
/// 
/// Used for merging several message to into one shot.
/// 
/// Max length: 9.
/// 
/// Example:
/// ```
/// context.send(&struct_sequence![message1, message2]).await.ok();
/// ```
// ----------------------------------------------------------------------------------------------------
#[macro_export]
macro_rules! struct_sequence {
    ($($s: expr),+) => {
        struct_sequence_name!($($s),+)($($s),+)
    };
}