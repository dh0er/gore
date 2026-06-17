# Inventory Add-Item Manual Test

## Test Scenario: Add Item to Inventory from Bundled Catalog

### Setup

1. Copy a real save slot to a test location.
2. Open goresave and load the test save.

### Steps

1. Navigate to the **Inventory** tab.
2. Verify that inventory items are grouped into collapsible categories (melee/ranged weapons, runes, scrolls, food, misc, trophies, writings, mission items, keys, amulets, other).
3. Expand and collapse categories to confirm functionality.
4. Verify that search filtering still works across categories.
5. Click the **"Add item"** button.
6. In the item picker, search for an item not currently in the save (e.g., `ItMi_Sulfur`).
7. Confirm the search returns results from the bundled catalog (798 total Gothic 1 Remake item IDs).
8. Set the item count to `7`.
9. Click **Add** to add the item to the inventory.
10. Click **Save** to write the changes to the save file (backup is created automatically).

### Verification

1. **goresave**: Close and reopen the save file in goresave.
   - Confirm the newly added item (e.g., `ItMi_Sulfur` with count 7) is listed in the Inventory tab.
   - Confirm it appears in the correct category.

2. **Gothic 1 Remake in-game**:
   - Load the edited save in Gothic 1 Remake.
   - Verify the new item appears in the player's inventory with the correct count (7).
   - Confirm it displays correctly in the game's UI.

3. **Round-trip consistency**:
   - Save the game in Gothic 1 Remake.
   - Close Gothic 1 Remake.
   - Reopen the new save in goresave.
   - Confirm the item is still present and the count is consistent.

### Implementation Details

- **Core operation**: `private.inventory.addItem` handles adding items with full payload validation before any write.
- **Bundled catalog**: Generated from UE4SS object dump via `tools/build_item_catalog.py` (798 items).
- **Safety**: All inventory edits go through the standard backup-and-validate workflow; saves are rejected if validation fails.
