fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
}

fn read_payload_chunks(
    payload_reader: &mut dyn CasPayloadReader,
) -> Result<Vec<Vec<u8>>, ObjectStoreError> {
    let mut chunks = Vec::new();
    while let Some(chunk) = payload_reader.read_next_chunk()? {
        if chunk.len() > MAX_PAYLOAD_CHUNK_BYTES {
            return Err(ObjectStoreError::InvalidPayload);
        }
        chunks.push(chunk);
    }
    CasPayload::from_chunks(&chunks)?;
    Ok(chunks)
}

fn stored_record_matches_put(record: &CasObjectRecord, request: &CasPutRequest) -> bool {
    record.address == request.address
        && record.size_bytes == request.payload.total_size_bytes
        && record.kms_boundary == request.kms_boundary
        && record.worm_policy == request.worm_policy
        && record.audit_anchor == request.audit_anchor
        && record.durability == request.durability
        && record.user_metadata == request.user_metadata
}

fn is_valid_reference(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && value == trimmed
        && value.len() <= MAX_REFERENCE_LEN
        && !trimmed.bytes().any(|byte| byte.is_ascii_control())
}

