import { test, expect } from '@playwright/test';
import {
  drawSelectionRectangle,
  dragHandle,
  waitForSVGReady,
  getSVGOffset,
  dragFromTo,
  startDragHandle,
  releaseMouse,
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
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);

      await polygon1.click();

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

    test('TC-18: Polygon border changes on hover', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const offset = await getSVGOffset(page);
      const polygon0 = svg.locator('polygon').nth(0);

      // Initially, polygon should have black stroke and strokeWidth 1
      await expect(polygon0).toHaveAttribute('stroke', 'black');
      await expect(polygon0).toHaveAttribute('stroke-width', '1');

      // Hover over polygon 0 (red triangle at ~245, 235)
      await page.mouse.move(offset.x + 245, offset.y + 235);

      // Verify stroke is blue (#3b82f6) and strokeWidth is 2
      await expect(polygon0).toHaveAttribute('stroke', '#3b82f6');
      await expect(polygon0).toHaveAttribute('stroke-width', '2');

      // Move mouse away to empty space
      await page.mouse.move(offset.x + 50, offset.y + 50);

      // Verify stroke returns to black and strokeWidth returns to 1
      await expect(polygon0).toHaveAttribute('stroke', 'black');
      await expect(polygon0).toHaveAttribute('stroke-width', '1');
    });
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

  test.describe('Hover Tests', () => {
    test('TC-17: Hovering over polygon shows blue border with width 2', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);

      // Verify initial state (black border, width 1)
      expect(await polygon1.getAttribute('stroke')).toBe('black');
      expect(await polygon1.getAttribute('stroke-width')).toBe('1');

      // Hover over polygon
      await polygon1.hover();

      // Verify hover state (blue border, width 2)
      expect(await polygon1.getAttribute('stroke')).toBe('#3b82f6');
      expect(await polygon1.getAttribute('stroke-width')).toBe('2');
    });

    test('TC-18: Hovering away from polygon clears hover effect', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);

      // Hover over polygon
      await polygon1.hover();
      expect(await polygon1.getAttribute('stroke')).toBe('#3b82f6');

      // Hover away to empty space
      const offset = await getSVGOffset(page);
      await page.mouse.move(offset.x + 50, offset.y + 50);

      // Verify hover cleared
      expect(await polygon1.getAttribute('stroke')).toBe('black');
      expect(await polygon1.getAttribute('stroke-width')).toBe('1');
    });

    test('TC-19: Clicking on polygon clears hover effect', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);

      // Hover over polygon
      await polygon1.hover();
      expect(await polygon1.getAttribute('stroke')).toBe('#3b82f6');

      // Click on polygon
      await polygon1.click();

      // Verify hover cleared (even though polygon is selected)
      expect(await polygon1.getAttribute('stroke')).toBe('black');
      expect(await polygon1.getAttribute('stroke-width')).toBe('1');
    });

    test('TC-20: Starting selection rectangle clears hover effect', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);

      // Hover over polygon
      await polygon1.hover();
      expect(await polygon1.getAttribute('stroke')).toBe('#3b82f6');

      // Start drawing selection rectangle
      const offset = await getSVGOffset(page);
      await page.mouse.move(offset.x + 50, offset.y + 50);
      await page.mouse.down();

      // Verify hover cleared
      expect(await polygon1.getAttribute('stroke')).toBe('black');
      expect(await polygon1.getAttribute('stroke-width')).toBe('1');

      // Clean up
      await page.mouse.up();
    });

    test('TC-21: Hover works on multiple polygons independently', async ({ page }) => {
      const svg = page.locator('[data-testid="main-canvas"]');
      const polygon1 = svg.locator('polygon').nth(0);
      const polygon2 = svg.locator('polygon').nth(1);

      // Hover over first polygon
      await polygon1.hover();
      expect(await polygon1.getAttribute('stroke')).toBe('#3b82f6');
      expect(await polygon2.getAttribute('stroke')).toBe('black');

      // Hover over second polygon
      await polygon2.hover();
      expect(await polygon1.getAttribute('stroke')).toBe('black');
      expect(await polygon2.getAttribute('stroke')).toBe('#3b82f6');
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
      const svg = page.locator('svg');

      // Make changes
      await drawSelectionRectangle(page, 220, 210, 310, 310);
      const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
      const box = await selectionBox.boundingBox();
      if (!box) throw new Error('Box not found');
      await dragFromTo(page, box.x + 35, box.y + 40, box.x + 85, box.y + 70);

      // Click reset
      await page.click('button:has-text("Reset")');

      // Verify back to initial state
      await assertNoSelection(page);

      // Verify polygons are at initial positions
      const polygon1 = svg.locator('polygon').nth(0);
      const points = await polygon1.getAttribute('points');
      expect(points).toBe('230,220 260,220 245,250');
    });
  });
});
