use gateway_auth::{now_unix, Principal};
use gateway_core::{GatewayError, GatewayResult, ObjectIdCodec, ObjectKind};
use gateway_storage::{Store, StoredObjectRef};
use rand::{rngs::OsRng, RngCore};

const IDENTITY_SOURCE: &str = "cypht.saved-search.identity";
const INDEX_SOURCE: &str = "cypht.saved-search.name-index";
const ACTIVE: &str = "active";
const TOMBSTONE: &str = "deleted";

pub(super) fn ensure_saved_search_id(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    name: &str,
) -> GatewayResult<String> {
    let lookup_id = ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        INDEX_SOURCE,
        [name],
    )?;
    if let Some(index) = store.get_object_ref(
        &principal.username,
        ObjectKind::SavedSearch.as_str(),
        &lookup_id,
    )? {
        let parts = auth.open_object_id_parts(&index.parts_ciphertext)?;
        if parts.len() == 2 && parts[0] == INDEX_SOURCE && parts[1] == TOMBSTONE {
            return create_saved_search_id(ids, auth, store, principal, name, &lookup_id);
        }
        if parts.len() != 2 || parts[0] != INDEX_SOURCE {
            return Err(GatewayError::Crypto);
        }
        if saved_search_id_name(ids, auth, store, principal, &parts[1])?.as_deref() == Some(name) {
            return Ok(parts[1].clone());
        }
        return create_saved_search_id(ids, auth, store, principal, name, &lookup_id);
    }
    create_saved_search_id(ids, auth, store, principal, name, &lookup_id)
}

fn create_saved_search_id(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    name: &str,
    lookup_id: &str,
) -> GatewayResult<String> {
    let mut random = [0_u8; 32];
    OsRng.fill_bytes(&mut random);
    let generation = hex::encode(random);
    random.fill(0);
    let id = ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        IDENTITY_SOURCE,
        [&generation],
    )?;
    let identity = object_ref(
        auth,
        principal,
        &id,
        vec![
            IDENTITY_SOURCE.into(),
            generation,
            ACTIVE.into(),
            name.into(),
        ],
    )?;
    let index = object_ref(
        auth,
        principal,
        lookup_id,
        vec![INDEX_SOURCE.into(), id.clone()],
    )?;
    store.put_object_refs(&[identity, index])?;
    Ok(id)
}

pub(super) fn saved_search_id_name(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    id: &str,
) -> GatewayResult<Option<String>> {
    let decoded = ids.decode_versioned(ObjectKind::SavedSearch, id)?;
    if decoded.version != 2 || decoded.parts.len() != 1 {
        return Err(GatewayError::NotFound("saved search".into()));
    }
    let Some(reference) =
        store.get_object_ref(&principal.username, ObjectKind::SavedSearch.as_str(), id)?
    else {
        return Err(GatewayError::NotFound("saved search".into()));
    };
    let parts = auth.open_object_id_parts(&reference.parts_ciphertext)?;
    if parts.len() != 4 || parts[0] != IDENTITY_SOURCE {
        return Err(GatewayError::NotFound("saved search".into()));
    }
    if ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        IDENTITY_SOURCE,
        [&parts[1]],
    )? != id
    {
        return Err(GatewayError::NotFound("saved search".into()));
    }
    if parts[2] != ACTIVE {
        return Ok(None);
    }
    Ok(Some(parts[3].clone()))
}

pub(super) fn retire_missing_saved_search_ids(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    live_names: &std::collections::HashSet<String>,
) -> GatewayResult<()> {
    let references =
        store.list_object_refs(&principal.username, ObjectKind::SavedSearch.as_str())?;
    let mut tombstones = Vec::new();
    for reference in references {
        let parts = auth.open_object_id_parts(&reference.parts_ciphertext)?;
        if parts.len() != 4 || parts[0] != IDENTITY_SOURCE || parts[2] != ACTIVE {
            continue;
        }
        if ids.encode_private(
            ObjectKind::SavedSearch,
            &principal.username,
            IDENTITY_SOURCE,
            [&parts[1]],
        )? != reference.public_id
            || live_names.contains(&parts[3])
        {
            continue;
        }
        let identity = object_ref(
            auth,
            principal,
            &reference.public_id,
            vec![
                IDENTITY_SOURCE.into(),
                parts[1].clone(),
                TOMBSTONE.into(),
                parts[3].clone(),
            ],
        )?;
        let lookup_id = ids.encode_private(
            ObjectKind::SavedSearch,
            &principal.username,
            INDEX_SOURCE,
            [&parts[3]],
        )?;
        let index = object_ref(
            auth,
            principal,
            &lookup_id,
            vec![INDEX_SOURCE.into(), TOMBSTONE.into()],
        )?;
        tombstones.push(identity);
        tombstones.push(index);
    }
    store.put_object_refs(&tombstones)
}
pub(super) fn complete_saved_search_rename(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    id: &str,
    new_name: &str,
) -> GatewayResult<()> {
    let identity = store
        .get_object_ref(&principal.username, ObjectKind::SavedSearch.as_str(), id)?
        .ok_or_else(|| GatewayError::NotFound("saved search".into()))?;
    let parts = auth.open_object_id_parts(&identity.parts_ciphertext)?;
    if parts.len() != 4 || parts[0] != IDENTITY_SOURCE || parts[2] != TOMBSTONE {
        return Err(GatewayError::Conflict(
            "saved search identity is not pending rename".into(),
        ));
    }
    let old_name = parts[3].clone();
    let new_lookup_id = ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        INDEX_SOURCE,
        [new_name],
    )?;
    if let Some(index) = store.get_object_ref(
        &principal.username,
        ObjectKind::SavedSearch.as_str(),
        &new_lookup_id,
    )? {
        let index_parts = auth.open_object_id_parts(&index.parts_ciphertext)?;
        if index_parts.len() != 2 || index_parts[0] != INDEX_SOURCE {
            return Err(GatewayError::Crypto);
        }
        if index_parts[1] != TOMBSTONE && index_parts[1] != id {
            return Err(GatewayError::Conflict(
                "saved search name already exists".into(),
            ));
        }
    }
    let identity_update = object_ref(
        auth,
        principal,
        id,
        vec![
            IDENTITY_SOURCE.into(),
            parts[1].clone(),
            ACTIVE.into(),
            new_name.into(),
        ],
    )?;
    let new_index = object_ref(
        auth,
        principal,
        &new_lookup_id,
        vec![INDEX_SOURCE.into(), id.into()],
    )?;
    if old_name == new_name {
        return store.put_object_refs(&[identity_update, new_index]);
    }
    let old_lookup_id = ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        INDEX_SOURCE,
        [&old_name],
    )?;
    let old_tombstone = object_ref(
        auth,
        principal,
        &old_lookup_id,
        vec![INDEX_SOURCE.into(), TOMBSTONE.into()],
    )?;
    store.put_object_refs(&[identity_update, old_tombstone, new_index])
}
pub(super) fn retire_saved_search_id(
    ids: &ObjectIdCodec,
    auth: &gateway_auth::AuthService,
    store: &Store,
    principal: &Principal,
    id: &str,
) -> GatewayResult<String> {
    let name = saved_search_id_name(ids, auth, store, principal, id)?
        .ok_or_else(|| GatewayError::NotFound("saved search".into()))?;
    let identity = store
        .get_object_ref(&principal.username, ObjectKind::SavedSearch.as_str(), id)?
        .ok_or_else(|| GatewayError::NotFound("saved search".into()))?;
    let parts = auth.open_object_id_parts(&identity.parts_ciphertext)?;
    let identity_tombstone = object_ref(
        auth,
        principal,
        id,
        vec![
            IDENTITY_SOURCE.into(),
            parts[1].clone(),
            TOMBSTONE.into(),
            name.clone(),
        ],
    )?;
    let lookup_id = ids.encode_private(
        ObjectKind::SavedSearch,
        &principal.username,
        INDEX_SOURCE,
        [&name],
    )?;
    let index_tombstone = object_ref(
        auth,
        principal,
        &lookup_id,
        vec![INDEX_SOURCE.into(), TOMBSTONE.into()],
    )?;
    store.put_object_refs(&[identity_tombstone, index_tombstone])?;
    Ok(name)
}

fn object_ref(
    auth: &gateway_auth::AuthService,
    principal: &Principal,
    public_id: &str,
    parts: Vec<String>,
) -> GatewayResult<StoredObjectRef> {
    Ok(StoredObjectRef {
        public_id: public_id.into(),
        owner: principal.username.clone(),
        kind: ObjectKind::SavedSearch.as_str().into(),
        parts_ciphertext: auth.seal_object_id_parts(&parts)?,
        created_at: now_unix(),
    })
}

#[cfg(test)]
#[path = "saved_search_ids_tests.rs"]
mod tests;
