#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, StructOfArray)]
#[soa_derive = "Debug, Clone, PartialEq"]
pub struct Particle {
    #[soa_derive(zip)]
    pub name: String,
    #[soa_derive(zip)]
    pub mass: f64,
}

impl Particle {
    pub fn new(name: String, mass: f64) -> Self {
        Particle {
            name: name,
            mass: mass,
        }
    }
}
