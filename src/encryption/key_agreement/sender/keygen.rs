use ml_kem::{
    Kem, MlKem1024,
    ml_kem_1024::{DecapsulationKey, EncapsulationKey},
};

struct KeyPair {
    private_key: DecapsulationKey,
    public_key: EncapsulationKey,
}

impl KeyPair {
    pub fn generate_pair() -> Self {
        let (private_key, public_key) = MlKem1024::generate_keypair();

        KeyPair { private_key, public_key }
    }
}

