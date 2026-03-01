/// Owned data for a br_table instruction.
/// wasmparser's BrTable<'a> contains a BinaryReader with a lifetime,
/// so we convert it to this owned representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrTableData {
    /// Branch target labels (excluding the default).
    pub targets: Vec<u32>,
    /// The default branch target label.
    pub default: u32,
}

// ---- Instruction enum generation ----
// We use a tt-muncher to process each operator one at a time,
// accumulating enum variants. Only BrTable needs special handling
// (its wasmparser type has a lifetime). All other types (MemArg,
// Ordering, TryTable, ResumeTable, etc.) are 'static and used directly.

macro_rules! define_instruction {
    ($($all:tt)*) => {
        define_instruction_inner!(@enum [] $($all)*);
    };
}

macro_rules! define_instruction_inner {
    // Terminal: emit the enum with all accumulated variants
    (@enum [$($variants:tt)*]) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        #[allow(missing_docs, non_camel_case_types)]
        pub enum Instruction {
            $($variants)*
        }
    };

    // Special case: BrTable (has lifetime that must be replaced)
    (@enum [$($v:tt)*] @$p:ident BrTable { $arg:ident : $t:ty } => $visit:ident $ann:tt $($rest:tt)*) => {
        define_instruction_inner!(@enum [$($v)* BrTable { $arg: BrTableData },] $($rest)*);
    };

    // Generic with fields (types pass through verbatim - they're all 'static)
    (@enum [$($v:tt)*] @$p:ident $op:ident { $($arg:ident : $argty:ty),* } => $visit:ident $ann:tt $($rest:tt)*) => {
        define_instruction_inner!(@enum [$($v)* $op { $($arg: $argty),* },] $($rest)*);
    };

    // No fields
    (@enum [$($v:tt)*] @$p:ident $op:ident => $visit:ident $ann:tt $($rest:tt)*) => {
        define_instruction_inner!(@enum [$($v)* $op,] $($rest)*);
    };
}

wasmparser::for_each_operator!(define_instruction);

// ---- Operator -> Instruction conversion ----

macro_rules! define_from_operator {
    ($( @$proposal:ident $op:ident $({ $($arg:ident : $argty:ty),* })? => $visit:ident ($($ann:tt)*) )*) => {
        #[allow(missing_docs)]
        impl Instruction {
            pub fn from_operator(op: &wasmparser::Operator<'_>) -> Self {
                match op {
                    $(
                        wasmparser::Operator::$op $({ $($arg),* })? => {
                            define_from_operator!(@build $op $($($arg),*)?)
                        }
                    )*
                    _ => unreachable!("unknown operator"),
                }
            }
        }
    };

    // BrTable: convert from wasmparser::BrTable<'a> to BrTableData
    (@build BrTable $targets:ident) => {
        Instruction::BrTable {
            targets: BrTableData {
                targets: $targets.targets().map(|t| t.unwrap()).collect(),
                default: $targets.default(),
            }
        }
    };
    // No fields
    (@build $op:ident) => {
        Instruction::$op
    };
    // Generic fields: clone all (works for both Copy and non-Copy types)
    (@build $op:ident $($arg:ident),+) => {
        Instruction::$op { $($arg: $arg.clone()),+ }
    };
}

wasmparser::for_each_operator!(define_from_operator);
