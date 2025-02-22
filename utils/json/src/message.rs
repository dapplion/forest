// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

pub mod json {
    use base64::{prelude::BASE64_STANDARD, Engine};
    use cid::Cid;
    use forest_message::Message as MessageTrait;
    use forest_shim::{address::Address, econ::TokenAmount, message::Message};
    use fvm_ipld_encoding::Cbor;
    use fvm_ipld_encoding3::RawBytes;
    use fvm_shared3::message::Message as Message_v3;
    use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

    use crate::address::json::AddressJson;

    /// Wrapper for serializing and de-serializing a Message from JSON.
    #[derive(Deserialize, Serialize, Debug)]
    #[serde(transparent)]
    pub struct MessageJson(#[serde(with = "self")] pub Message);

    /// Wrapper for serializing a Message reference to JSON.
    #[derive(Serialize)]
    #[serde(transparent)]
    pub struct MessageJsonRef<'a>(#[serde(with = "self")] pub &'a Message);

    impl From<MessageJson> for Message {
        fn from(wrapper: MessageJson) -> Self {
            wrapper.0
        }
    }

    impl From<Message> for MessageJson {
        fn from(wrapper: Message) -> Self {
            MessageJson(wrapper)
        }
    }

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct JsonHelper {
        version: u64,
        to: AddressJson,
        from: AddressJson,
        #[serde(rename = "Nonce")]
        sequence: u64,
        #[serde(with = "crate::token_amount::json")]
        value: TokenAmount,
        gas_limit: u64,
        #[serde(with = "crate::token_amount::json")]
        gas_fee_cap: TokenAmount,
        #[serde(with = "crate::token_amount::json")]
        gas_premium: TokenAmount,
        #[serde(rename = "Method")]
        method_num: u64,
        params: Option<String>,
        #[serde(default, rename = "CID", with = "crate::cid::opt")]
        cid: Option<Cid>,
    }

    pub fn serialize<S>(m: &Message, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        JsonHelper {
            version: m.version,
            to: Address::from(m.to).into(),
            from: Address::from(m.from).into(),
            sequence: m.sequence,
            value: m.value(),
            gas_limit: m.gas_limit(),
            gas_fee_cap: m.gas_fee_cap(),
            gas_premium: m.gas_premium(),
            method_num: m.method_num,
            params: Some(BASE64_STANDARD.encode(m.params.bytes())),
            cid: Some(m.cid().map_err(ser::Error::custom)?),
        }
        .serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Message, D::Error>
    where
        D: Deserializer<'de>,
    {
        let m: JsonHelper = Deserialize::deserialize(deserializer)?;
        Ok(Message_v3 {
            version: m.version,
            to: Address::from(m.to).into(),
            from: Address::from(m.from).into(),
            sequence: m.sequence,
            value: m.value.into(),
            gas_limit: m.gas_limit,
            gas_fee_cap: m.gas_fee_cap.into(),
            gas_premium: m.gas_premium.into(),
            method_num: m.method_num,
            params: RawBytes::new(
                BASE64_STANDARD
                    .decode(m.params.unwrap_or_default())
                    .map_err(de::Error::custom)?,
            ),
        }
        .into())
    }

    pub mod vec {
        use forest_utils::json::GoVecVisitor;
        use serde::ser::SerializeSeq;

        use super::*;

        pub fn serialize<S>(m: &[Message], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(m.len()))?;
            for e in m {
                seq.serialize_element(&MessageJsonRef(e))?;
            }
            seq.end()
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Message>, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(GoVecVisitor::<Message, MessageJson>::new())
        }
    }
}

#[cfg(test)]
pub mod tests {
    use forest_shim::{
        address::Address,
        econ::TokenAmount,
        message::{Message, Message_v3},
    };
    use quickcheck_macros::quickcheck;
    use serde_json;

    use super::json::{MessageJson, MessageJsonRef};

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct MessageWrapper {
        pub message: Message,
    }

    impl quickcheck::Arbitrary for MessageWrapper {
        fn arbitrary(g: &mut quickcheck::Gen) -> Self {
            let msg = Message_v3 {
                to: Address::new_id(u64::arbitrary(g)).into(),
                from: Address::new_id(u64::arbitrary(g)).into(),
                version: u64::arbitrary(g),
                sequence: u64::arbitrary(g),
                value: TokenAmount::from_atto(u64::arbitrary(g)).into(),
                method_num: u64::arbitrary(g),
                params: fvm_ipld_encoding3::RawBytes::new(Vec::arbitrary(g)),
                gas_limit: u64::arbitrary(g),
                gas_fee_cap: TokenAmount::from_atto(u64::arbitrary(g)).into(),
                gas_premium: TokenAmount::from_atto(u64::arbitrary(g)).into(),
            };
            MessageWrapper {
                message: msg.into(),
            }
        }
    }

    #[quickcheck]
    fn message_roundtrip(message: MessageWrapper) {
        let serialized = serde_json::to_string(&MessageJsonRef(&message.message)).unwrap();
        let parsed: MessageJson = serde_json::from_str(&serialized).unwrap();
        assert_eq!(message.message, parsed.0);
    }
}
