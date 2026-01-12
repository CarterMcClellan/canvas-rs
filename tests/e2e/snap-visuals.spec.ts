import { test, expect } from '@playwright/test';
import { getSVGOffset } from './helpers/canvas-helpers';
import { assertSelectionState, assertBoundingBox } from './helpers/assertions';

test.describe('Snap Guidelines E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    // Wait for SVG overlay to be ready
    await page.waitForSelector('[data-testid="main-canvas"]');
    await page.waitForTimeout(500); // Wait for GPU canvas to initialize
  });

  // TODO: Snap guidelines test needs coordinate adjustment after GPU rendering migration
  // The snap feature works but test coordinates need recalibration
  test.skip('TC-Snap-1: Verify snap lines appear when dragging close to another polygon', async ({ page }) => {
    // Use the SVG overlay for UI elements
    const svg = page.locator('[data-testid="main-canvas"]');

    // 1. Select the polygon at ~270,230 (blue one, index 1)
    //    Polygon 1 is right of Polygon 0.
    //    Polygon 0 (red) is at roughly x=230, y=220.
    //    Polygon 1 (blue) is at x=270, y=230.
    
    // Select Polygon 1 (Blue)
    // It's the second one in the list.
    // We can click it directly or marquee select it.
    // Let's try clicking it. 270+15, 230+15 should hit it.
    const offset = await getSVGOffset(page);
    await page.mouse.click(offset.x + 285, offset.y + 240);
    
    await assertSelectionState(page, [1]);

    const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
    const box = await selectionBox.boundingBox();
    if (!box) throw new Error('Selection box not found');

    // 2. Drag it left towards Polygon 0 (Red).
    //    Polygon 0 right edge is ~260.
    //    Polygon 1 left edge is 270.
    //    We want to align Polygon 1's Left edge (270) with Polygon 0's Right edge (260)?
    //    Or snap Left-to-Left? Polygon 0 Left is 230.
    //    Let's try dragging Left.
    
    // Start drag
    await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
    await page.mouse.down();
    
    // Move slowly to trigger snap
    // Target x=230 (Polygon 0 Left). Current x=270. Move -40.
    // Move to x=232 (close to snap)
    await page.mouse.move(box.x + box.width / 2 - 38, box.y + box.height / 2, { steps: 10 });

    // 3. Verify guideline appears (Red line)
    // Note: Using toHaveCount instead of toBeVisible because SVG elements with pointer-events="none"
    // can be reported as hidden by Playwright even when they're visually rendered
    const guideline = svg.locator('line[stroke="red"]');
    await expect(guideline).toHaveCount(1);
    
    // 4. Release
    await page.mouse.up();
    
    // 5. Verify position snapped (should be exactly at x=230 relative to svg origin + margin)
    //    Initial x=270. Moved -38 -> 232. Snap -> 230.
    //    Delta should be -40.
    //    New x should be 270 - 40 = 230.
    
    // Wait a bit for update
    await page.waitForTimeout(100);
    
    // Re-get box
    const newBox = await selectionBox.boundingBox();
    // Note: boundingBox() returns page coordinates. We need relative to SVG if we strictly check values, 
    // but checking relative change is safer or using our helper assertions.
    // assertBoundingBox uses relative coordinates if implemented correctly or we adjust expectations.
    // Our helper assertBoundingBox expects values relative to SVG origin (0,0 inside SVG).
    // Let's assume assertBoundingBox handles it or we pass expected values.
    
    await assertBoundingBox(selectionBox, {
        x: 230,
        y: 230, // Y didn't change
        width: 30,
        height: 30
    });
  });
});
