use crate::error::{LucetcError, LucetcErrorKind};
use cranelift_codegen::{isa, settings::Configurable};
use failure::{format_err, ResultExt};
use std::collections::HashMap;
use target_lexicon::Triple;

/// x86 CPU families used as shorthand for different CPU feature configurations.
///
/// Matches the definitions from `cranelift-codegen`'s x86 settings definition.
#[derive(Debug, Clone, Copy)]
pub enum TargetCpu {
    Native,
    Baseline,
    Nehalem,
    Sandybridge,
    Haswell,
    Broadwell,
    Skylake,
    Cannonlake,
    Icelake,
    Znver1,
}

impl TargetCpu {
    fn features(&self) -> Vec<SpecificFeature> {
        use SpecificFeature::*;
        use TargetCpu::*;
        match self {
            Native | Baseline => vec![],
            Nehalem => vec![SSE3, SSSE3, SSE41, SSE42, Popcnt],
            // Note: this is not part of the Cranelift profile for Haswell, and there is no Sandy
            // Bridge profile. Instead, Cranelift only uses CPUID detection to enable AVX. If we
            // want to bypass CPUID when compiling, we need to set AVX manually, and Sandy Bridge is
            // the first family of Intel CPUs with AVX.
            Sandybridge => [Nehalem.features().as_slice(), &[AVX]].concat(),
            Haswell => [Sandybridge.features().as_slice(), &[BMI1, BMI2, Lzcnt]].concat(),
            Broadwell => Haswell.features(),
            Skylake => Broadwell.features(),
            Cannonlake => Skylake.features(),
            Icelake => Cannonlake.features(),
            Znver1 => vec![SSE3, SSSE3, SSE41, SSE42, Popcnt, AVX, BMI1, BMI2, Lzcnt],
        }
    }
}

/// Individual CPU features that may be used during codegen.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SpecificFeature {
    SSE3,
    SSSE3,
    SSE41,
    SSE42,
    Popcnt,
    AVX,
    BMI1,
    BMI2,
    Lzcnt,
}

/// An x86-specific configuration of CPU features that affect code generation.
#[derive(Debug, Clone)]
pub struct CpuFeatures {
    /// Base CPU profile to use
    cpu: TargetCpu,
    /// Specific CPU features to add or remove from the profile
    specific_features: HashMap<SpecificFeature, bool>,
}

impl Default for CpuFeatures {
    fn default() -> Self {
        Self::detect_cpuid()
    }
}

impl CpuFeatures {
    pub fn new(cpu: TargetCpu, specific_features: HashMap<SpecificFeature, bool>) -> Self {
        Self {
            cpu,
            specific_features,
        }
    }

    /// Return a `CpuFeatures` that uses the CPUID instruction to determine which features to enable.
    pub fn detect_cpuid() -> Self {
        CpuFeatures {
            cpu: TargetCpu::Native,
            specific_features: HashMap::new(),
        }
    }

    /// Return a `CpuFeatures` with no optional features enabled.
    pub fn baseline() -> Self {
        CpuFeatures {
            cpu: TargetCpu::Baseline,
            specific_features: HashMap::new(),
        }
    }

    pub fn set(&mut self, sf: SpecificFeature, enabled: bool) {
        self.specific_features.insert(sf, enabled);
    }

    /// Return a `cranelift_codegen::isa::Builder` configured with these CPU features.
    pub fn isa_builder(&self) -> Result<isa::Builder, LucetcError> {
        use SpecificFeature::*;
        use TargetCpu::*;

        let mut isa_builder = if let Native = self.cpu {
            cranelift_native::builder()
                .map_err(|_| format_err!("host machine is not a supported target"))
        } else {
            isa::lookup(Triple::host())
                .map_err(|_| format_err!("host machine is not a supported target"))
        }
        .context(LucetcErrorKind::Unsupported)?;

        let mut specific_features = self.specific_features.clone();

        // add any features from the CPU profile if they are not already individually specified
        for cpu_feature in self.cpu.features() {
            specific_features.entry(cpu_feature).or_insert(true);
        }

        for (feature, enabled) in specific_features.into_iter() {
            let enabled = if enabled { "true" } else { "false" };
            match feature {
                SSE3 => isa_builder.set("has_sse3", enabled).unwrap(),
                SSSE3 => isa_builder.set("has_ssse3", enabled).unwrap(),
                SSE41 => isa_builder.set("has_sse41", enabled).unwrap(),
                SSE42 => isa_builder.set("has_sse42", enabled).unwrap(),
                Popcnt => isa_builder.set("has_popcnt", enabled).unwrap(),
                AVX => isa_builder.set("has_avx", enabled).unwrap(),
                BMI1 => isa_builder.set("has_bmi1", enabled).unwrap(),
                BMI2 => isa_builder.set("has_bmi2", enabled).unwrap(),
                Lzcnt => isa_builder.set("has_lzcnt", enabled).unwrap(),
            }
        }


        Ok(isa_builder)
    }
}
