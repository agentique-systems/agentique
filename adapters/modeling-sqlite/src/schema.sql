CREATE TABLE repository_metadata (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    repository_id TEXT NOT NULL,
    format TEXT NOT NULL CHECK (format = 'agentique-modeling-sqlite/1')
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    data BLOB NOT NULL,
    digest TEXT NOT NULL
);

CREATE TABLE revisions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    parent_id TEXT,
    manifest BLOB NOT NULL,
    digest TEXT NOT NULL,
    UNIQUE (project_id, id),
    FOREIGN KEY (project_id, parent_id) REFERENCES revisions(project_id, id)
);

CREATE TABLE blobs (
    digest TEXT PRIMARY KEY,
    bytes BLOB NOT NULL
);

CREATE TABLE revision_documents (
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    document_id TEXT NOT NULL,
    path TEXT NOT NULL,
    source_revision_id TEXT NOT NULL,
    language TEXT NOT NULL,
    blob_digest TEXT NOT NULL REFERENCES blobs(digest),
    PRIMARY KEY (revision_id, document_id),
    UNIQUE (revision_id, path)
);

CREATE TABLE source_revisions (
    source_revision_id TEXT PRIMARY KEY,
    document_id TEXT NOT NULL,
    language TEXT NOT NULL,
    blob_digest TEXT NOT NULL REFERENCES blobs(digest)
);

CREATE TABLE branches (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id),
    name TEXT NOT NULL,
    head_id TEXT NOT NULL,
    data BLOB NOT NULL,
    digest TEXT NOT NULL,
    UNIQUE (project_id, name),
    FOREIGN KEY (project_id, head_id) REFERENCES revisions(project_id, id)
);

CREATE TABLE semantic_caches (
    revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
    blob_digest TEXT NOT NULL
);

CREATE TABLE operation_receipts (
    operation_id TEXT PRIMARY KEY,
    request_digest TEXT NOT NULL,
    revision_id TEXT NOT NULL REFERENCES revisions(id),
    data BLOB NOT NULL,
    digest TEXT NOT NULL
);

CREATE INDEX revision_parents ON revisions(project_id, parent_id);
CREATE INDEX document_blobs ON revision_documents(blob_digest);
