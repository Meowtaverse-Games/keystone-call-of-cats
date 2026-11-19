# Stage catalog and saves

`assets/stages/list.ron` defines ordered categories, explicit `unlock_after` stage
IDs, and each stage's `published` and initial `unlocked` flags. IDs are permanent;
never rename or reuse an assigned ID. Append new entries and add their embedded
map in `StageMeta::load_map`. Empty categories do not appear in the selector.

Published stages appear first within each category. Default builds also show
unpublished stages, with an experimental badge. `--no-default-features` excludes
them from selection, progression and next-stage lookup. Display numbers are
computed from this visible order, independently of IDs. Category gates point at
stable IDs, so appending a stage does not relock the following category.

Select categories with their tabs or Up/Down; change pages with the page buttons
or Left/Right. Enter plays the first visible unlocked card; 1/2/3 play the
corresponding visible card. Mouse and touch use the same button handlers. The last
played category and page are restored.

Use `--stage move-02` or `--stage=move-02`, including with
`--show-chunk-grammar-ascii-map`. Headless scenarios use `"stage": "move-02"`
or an unchanged `ron_path`. Numeric selectors are accepted only by save migration.

Progress saves contain a set of cleared IDs and an optional last played ID.
`src/resources/legacy_stage_ids.rs` is the shared frozen numeric mapping used by
save migration and the sheet exporter. Legacy `unlocked_until: N` becomes cleared
IDs 1 through N−1, except N >= 12 means all original twelve stages are cleared.
Zero and one mean no clears. Unknown last-played numbers are logged and omitted;
unknown script keys are retained in `stage_scripts.legacy-unmapped.ron`.

Migration preserves original bytes in numbered `.bak` files through `FileStorage`
before writing the replacement. Unreadable saves are likewise backed up before
default data can be persisted. A read/backup/migration write failure blocks writes
for that resource for the session; the next launch retries from the original.
Unmapped scripts must be saved successfully before replacing their source. A
successfully migrated save is read directly on subsequent launches.
