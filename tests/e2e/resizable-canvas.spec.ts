import { test, expect } from '@playwright/test';
import {
  drawSelectionRectangle,
  dragHandle,
  waitForSVGReady,
  getSVGOffset,
  dragFromTo,
  startDragHandle,
  releaseMouse,
  clickOnShape,
  hoverOnShape,
} from './helpers/canvas-helpers';
import {
  assertSelectionState,
  assertFlipState,
  assertBoundingBox,
  assertNoSelection,
} from './helpers/assertions';
import { INITIAL_BOUNDING_BOX } from './fixtures/expected-states';

test.describe('ResizableCanvas E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await waitForSVGReady(page);
  });

  test.describe('Selection Tests', () => {
    test('TC-1: Select all 3 polygons via marquee selection', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Draw selection rectangle around all polygons
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Verify all selected
      await assertSelectionState(page, [0, 1, 2]);

      // Verify bounding box
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
      await assertBoundingBox(selectionBox, INITIAL_BOUNDING_BOX);
    });

    test('TC-2: Select single polygon by clicking', async ({ page }) => {
      // Click on shape 0 (red triangle) using canvas coordinates
      await clickOnShape(page, 0);

      await assertSelectionState(page, [0]);
    });

    test('TC-3: Clear selection by clicking empty space', async ({ page }) => {
      // First select
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      await assertSelectionState(page, [0, 1, 2]);

      // Click empty space
      const offset = await getSVGOffset(page);
      await page.mouse.click(offset.x + 50, offset.y + 50);

      // Verify cleared
      await assertNoSelection(page);
    });

    test('TC-17: Verify preview box appears during marquee selection', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const offset = await getSVGOffset(page);

      // Start drawing marquee
      await page.mouse.move(offset.x + 210, offset.y + 210);
      await page.mouse.down();

      // Drag to cover Polygon 0
      await page.mouse.move(offset.x + 270, offset.y + 260, { steps: 5 });

      // Check for both marquee rect and preview box
      await expect(svg.locator('[data-testid="marquee-selection-rect"]')).toHaveCount(1);
      await expect(svg.locator('[data-testid="preview-bounding-box"]')).toHaveCount(1);

      await page.mouse.up();
    });

    // Note: TC-18 (Polygon border changes on hover) removed - SVG polygon attributes
    // don't exist in GPU rendering mode. Hover effects are visual only.
  });

  test.describe('Translation Tests', () => {
    test('TC-4: Translate selected polygons by dragging', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Select all
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Get selection box center
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
      const box = await selectionBox.boundingBox();
      if (!box) throw new Error('Selection box not found');

      const centerX = box.x + box.width / 2;
      const centerY = box.y + box.height / 2;

      // Drag to translate (50px right, 30px down)
      await dragFromTo(page, centerX, centerY, centerX + 50, centerY + 30);

      // Verify new position
      await assertBoundingBox(selectionBox, {
        x: 280, // 230 + 50
        y: 250, // 220 + 30
        width: 70,
        height: 80,
      });

      // Verify flip state unchanged
      await assertFlipState(page, false, false);
    });

    test('TC-5: Multiple translations accumulate correctly', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');

      // First translation
      let box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');
      await dragFromTo(page, box.x + 35, box.y + 40, box.x + 55, box.y + 60);

      // Second translation
      box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');
      await dragFromTo(page, box.x + 35, box.y + 40, box.x + 55, box.y + 60);

      // Verify cumulative translation
      await assertBoundingBox(selectionBox, {
        x: 270, // 230 + 20 + 20
        y: 260, // 220 + 20 + 20
        width: 70,
        height: 80,
      });
    });
  });

  test.describe('Complete Workflow', () => {
    test('TC-14: Full user workflow - select, translate, resize with inversions', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');

      // Step 1: Select all 3 polygons
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      await assertSelectionState(page, [0, 1, 2]);
      await assertFlipState(page, false, false);

      // Step 2: Translate
      let box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');
      await dragFromTo(
        page,
        box.x + box.width / 2,
        box.y + box.height / 2,
        box.x + box.width / 2 + 50,
        box.y + box.height / 2 + 30
      );
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);

      // Step 3: Invert X via right handle
      await dragHandle(page, 'right', -200, 0);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);

      // Step 4: Invert Y via bottom handle
      await dragHandle(page, 'bottom', 0, -200);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);

      // Step 5: Invert both via corner handle
      await dragHandle(page, 'bottom-right', -150, -150);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });
  });

  test.describe('Resize Handle Inversion Tests', () => {
    test('TC-6: Slowly invert from right handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag right handle leftward to invert (need to go past left edge: >70px)
      await startDragHandle(page, 'right', -100, 0);

      // During drag: flip state should be true for X axis
      await assertFlipState(page, true, false);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-7: Slowly invert from left handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag left handle rightward to invert (need to go past right edge: >70px)
      await startDragHandle(page, 'left', 100, 0);

      // During drag: flip state should be true for X axis
      await assertFlipState(page, true, false);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-8: Slowly invert from top handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag top handle downward to invert (need to go past bottom edge: >80px)
      await startDragHandle(page, 'top', 0, 110);

      // During drag: flip state should be true for Y axis
      await assertFlipState(page, false, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-9: Slowly invert from bottom handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag bottom handle upward to invert (need to go past top edge: >80px)
      await startDragHandle(page, 'bottom', 0, -110);

      // During drag: flip state should be true for Y axis
      await assertFlipState(page, false, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-10: Slowly invert from top-left handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag top-left handle down-right to invert both axes (>70px X, >80px Y)
      await startDragHandle(page, 'top-left', 100, 110);

      // During drag: flip state should be true for both axes
      await assertFlipState(page, true, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-11: Slowly invert from top-right handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag top-right handle down-left to invert both axes (>70px X, >80px Y)
      await startDragHandle(page, 'top-right', -100, 110);

      // During drag: flip state should be true for both axes
      await assertFlipState(page, true, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-12: Slowly invert from bottom-left handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag bottom-left handle up-right to invert both axes (>70px X, >80px Y)
      await startDragHandle(page, 'bottom-left', 100, -110);

      // During drag: flip state should be true for both axes
      await assertFlipState(page, true, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });

    test('TC-13: Slowly invert from bottom-right handle', async ({ page }) => {
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      // Slowly drag bottom-right handle up-left to invert both axes (>70px X, >80px Y)
      await startDragHandle(page, 'bottom-right', -100, -110);

      // During drag: flip state should be true for both axes
      await assertFlipState(page, true, true);
      await assertSelectionState(page, [0, 1, 2]);

      // Release and verify flip is committed to geometry
      await releaseMouse(page);
      await assertFlipState(page, false, false);
      await assertSelectionState(page, [0, 1, 2]);
    });
  });

  // Note: Hover Tests (TC-17 through TC-21) removed
  // With GPU rendering, shapes are rendered on canvas, not as SVG polygons.
  // Hover effects are visual only (cursor changes to pointer via is_shape_hovered prop).
  // Testing hover requires visual regression testing or checking cursor style.
  test.describe('Hover Tests', () => {
    test('TC-22: Hovering over shape changes cursor to pointer', async ({ page }) => {
      // Hover over shape 0 using canvas coordinates
      await hoverOnShape(page, 0);

      // Check canvas cursor style
      const canvas = page.locator('canvas');
      const cursor = await canvas.evaluate((el) => window.getComputedStyle(el).cursor);
      expect(cursor).toBe('pointer');
    });

    test('TC-23: Hovering over empty space has default cursor', async ({ page }) => {
      // Hover over empty space
      const offset = await getSVGOffset(page);
      await page.mouse.move(offset.x + 50, offset.y + 50);

      // Check canvas cursor style
      const canvas = page.locator('canvas');
      const cursor = await canvas.evaluate((el) => window.getComputedStyle(el).cursor);
      expect(cursor).toBe('default');
    });

    test('TC-24: Clicking on shape selects it', async ({ page }) => {
      // Click on shape 0
      await clickOnShape(page, 0);

      // Verify selection
      await assertSelectionState(page, [0]);
    });
  });

  test.describe('Edge Cases', () => {
    test('TC-15: Minimum size constraint enforced during resize', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Select all
      await drawSelectionRectangle(page, 220, 210, 310, 310);

      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
      const box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');

      // Try to resize to below minimum (MIN_SIZE = 10)
      await dragFromTo(page, box.x + box.width, box.y + box.height / 2, box.x + 5, box.y + box.height / 2);

      // Verify width is at minimum (10px)
      const width = parseFloat(await selectionBox.getAttribute('width') || '0');
      expect(width).toBeGreaterThanOrEqual(10);
    });

    test('TC-16: Reset button restores initial state', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Make changes
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
      const box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');
      await dragFromTo(page, box.x + 35, box.y + 40, box.x + 85, box.y + 70);

      // Click reset
      await page.click('button:has-text("Reset")');

      // Verify back to initial state - selection should be cleared
      await assertNoSelection(page);

      // Note: Can't verify polygon points with GPU rendering (shapes are on canvas, not SVG).
      // The reset functionality is verified by selection being cleared and shapes visually
      // returning to original positions (would require visual regression testing).
    });
  });

  test.describe('GPU Transform Regression Tests', () => {
    test('TC-25: Resize changes persist after release', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Select shapes
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');

      // Get initial bounding box
      const boxBefore = await selectionBox.boundingBox();
      if (!boxBefore) throw new Error('Selection box not found');
      const initialWidth = boxBefore.width;

      // Drag right handle to resize
      await dragHandle(page, 'right', 60, 0);

      // Get bounding box after resize
      const boxAfter = await selectionBox.boundingBox();
      if (!boxAfter) throw new Error('Selection box not found after resize');

      // Width should have increased by approximately 60px
      const widthDiff = boxAfter.width - initialWidth;
      expect(widthDiff, 'Resize should persist after release').toBeGreaterThan(50);
      expect(widthDiff, 'Resize should persist after release').toBeLessThan(70);

      // Deselect and reselect to verify changes are committed to shapes
      await page.mouse.click(100, 100);
      await page.waitForTimeout(200);

      // Reselect with larger area to capture resized shapes
      await drawSelectionRectangle(page, 220, 210, 380, 310);

      const boxReselected = await selectionBox.boundingBox();
      if (!boxReselected) throw new Error('Selection box not found after reselect');

      // Reselected width should be close to the resized width
      expect(Math.abs(boxReselected.width - boxAfter.width),
        'Resize should persist after deselect/reselect').toBeLessThan(10);
    });

    test('TC-26: Translation changes persist after release', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Select shapes
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');

      // Get initial bounding box position
      const boxBefore = await selectionBox.boundingBox();
      if (!boxBefore) throw new Error('Selection box not found');
      const initialX = boxBefore.x;
      const initialY = boxBefore.y;

      // Get center and translate
      const centerX = boxBefore.x + boxBefore.width / 2;
      const centerY = boxBefore.y + boxBefore.height / 2;
      await dragFromTo(page, centerX, centerY, centerX + 50, centerY + 40);

      // Get bounding box after translation
      const boxAfter = await selectionBox.boundingBox();
      if (!boxAfter) throw new Error('Selection box not found after translation');

      // Position should have changed by approximately the drag distance
      const xDiff = boxAfter.x - initialX;
      const yDiff = boxAfter.y - initialY;
      expect(xDiff, 'X translation should persist').toBeGreaterThan(40);
      expect(xDiff, 'X translation should persist').toBeLessThan(60);
      expect(yDiff, 'Y translation should persist').toBeGreaterThan(30);
      expect(yDiff, 'Y translation should persist').toBeLessThan(50);

      // Deselect and reselect to verify changes are committed
      await page.mouse.click(100, 100);
      await page.waitForTimeout(200);

      // Reselect - need to account for the translation
      await drawSelectionRectangle(page, 270, 250, 360, 350);

      const boxReselected = await selectionBox.boundingBox();
      if (!boxReselected) throw new Error('Selection box not found after reselect');

      // Reselected position should be close to the translated position
      expect(Math.abs(boxReselected.x - boxAfter.x),
        'X translation should persist after deselect/reselect').toBeLessThan(15);
      expect(Math.abs(boxReselected.y - boxAfter.y),
        'Y translation should persist after deselect/reselect').toBeLessThan(15);
    });

    test('TC-27: Corner resize maintains anchor at opposite corner', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');

      // Select shapes
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');

      // Get initial bounding box
      const boxBefore = await selectionBox.boundingBox();
      if (!boxBefore) throw new Error('Selection box not found');

      // Record the bottom-right corner (anchor for top-left resize)
      const anchorX = boxBefore.x + boxBefore.width;
      const anchorY = boxBefore.y + boxBefore.height;

      // Drag top-left handle to resize (towards top-left, making selection bigger)
      await dragHandle(page, 'top-left', -30, -30);

      // Get bounding box after resize
      const boxAfter = await selectionBox.boundingBox();
      if (!boxAfter) throw new Error('Selection box not found after resize');

      // Bottom-right corner should remain approximately the same
      const newAnchorX = boxAfter.x + boxAfter.width;
      const newAnchorY = boxAfter.y + boxAfter.height;

      expect(Math.abs(newAnchorX - anchorX),
        'Bottom-right X should stay anchored').toBeLessThan(10);
      expect(Math.abs(newAnchorY - anchorY),
        'Bottom-right Y should stay anchored').toBeLessThan(10);
    });
  });
});
