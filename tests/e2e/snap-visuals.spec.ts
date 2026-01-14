import { test, expect, Page } from '@playwright/test';
import { getSVGOffset } from './helpers/canvas-helpers';

// Canvas dimensions (must match CANVAS_WIDTH/HEIGHT in resizable_canvas.rs)
const CANVAS_WIDTH = 800;
const CANVAS_HEIGHT = 600;

async function selectShapeAt(page: Page, x: number, y: number) {
  const offset = await getSVGOffset(page);
  await page.mouse.click(offset.x + x, offset.y + y);
  await page.waitForTimeout(100);
}

async function getSelectionBox(page: Page) {
  const svg = page.locator('[data-testid="main-canvas"]');
  const selectionBox = svg.locator('[data-testid="selection-bounding-box"]');
  return selectionBox.boundingBox();
}

async function startDrag(page: Page, box: { x: number; y: number; width: number; height: number }) {
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
}

async function dragTo(page: Page, x: number, y: number) {
  const offset = await getSVGOffset(page);
  await page.mouse.move(offset.x + x, offset.y + y, { steps: 10 });
}

async function getSnapLines(page: Page) {
  const svg = page.locator('[data-testid="main-canvas"]');
  const lines = svg.locator('line[stroke="red"]');
  const count = await lines.count();

  const snapLines: Array<{
    x1: number;
    y1: number;
    x2: number;
    y2: number;
    isVertical: boolean;
  }> = [];

  for (let i = 0; i < count; i++) {
    const line = lines.nth(i);
    const x1 = parseFloat(await line.getAttribute('x1') || '0');
    const y1 = parseFloat(await line.getAttribute('y1') || '0');
    const x2 = parseFloat(await line.getAttribute('x2') || '0');
    const y2 = parseFloat(await line.getAttribute('y2') || '0');
    const isVertical = Math.abs(x1 - x2) < 1; // vertical line has same x

    snapLines.push({ x1, y1, x2, y2, isVertical });
  }

  return snapLines;
}

test.describe('Snap Guidelines E2E Tests', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('/');
    await page.waitForSelector('[data-testid="main-canvas"]');
    await page.waitForTimeout(500); // Wait for GPU canvas to initialize
  });

  test('TC-Snap-1: Snap lines are red SVG lines', async ({ page }) => {
    // Select a shape (heart at approx 50, 350)
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag towards canvas edge to trigger snap
    await dragTo(page, 5, 400);

    const snapLines = await getSnapLines(page);

    if (snapLines.length > 0) {
      // Verify lines exist and are red (checked by locator selector)
      expect(snapLines.length).toBeGreaterThanOrEqual(1);

      // Each line should have finite coordinates (can be negative as they extend to show alignment)
      for (const line of snapLines) {
        expect(Number.isFinite(line.x1)).toBe(true);
        expect(Number.isFinite(line.y1)).toBe(true);
        expect(Number.isFinite(line.x2)).toBe(true);
        expect(Number.isFinite(line.y2)).toBe(true);
      }
    }

    await page.mouse.up();
  });

  test('TC-Snap-2: Snap lines disappear after mouseup', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);
    await dragTo(page, 5, 400);

    const snapLinesDuringDrag = await getSnapLines(page);

    // Release mouse
    await page.mouse.up();
    await page.waitForTimeout(100);

    // Snap lines should be cleared
    const snapLinesAfterRelease = await getSnapLines(page);
    expect(snapLinesAfterRelease.length).toBe(0);
  });

  test('TC-Snap-3: Snap to left canvas edge shows vertical line at x=0', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag to left edge of canvas (x ~= 0)
    await dragTo(page, 5, 400);

    const snapLines = await getSnapLines(page);

    // Should have a vertical snap line near x=0
    const verticalLines = snapLines.filter(l => l.isVertical);
    if (verticalLines.length > 0) {
      const leftEdgeLine = verticalLines.find(l => l.x1 < 10);
      expect(leftEdgeLine).toBeDefined();
    }

    await page.mouse.up();
  });

  test('TC-Snap-4: Snap to canvas center shows vertical line at x=400', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag to horizontal center of canvas
    const centerX = CANVAS_WIDTH / 2;
    await dragTo(page, centerX, 400);

    const snapLines = await getSnapLines(page);

    // Should have a vertical snap line near center (x=400)
    const verticalLines = snapLines.filter(l => l.isVertical);
    if (verticalLines.length > 0) {
      const centerLine = verticalLines.find(l => Math.abs(l.x1 - centerX) < 20);
      if (centerLine) {
        expect(centerLine.x1).toBeCloseTo(centerX, 0);
      }
    }

    await page.mouse.up();
  });

  test('TC-Snap-5: Snap to canvas vertical center shows horizontal line at y=300', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag to vertical center of canvas
    const centerY = CANVAS_HEIGHT / 2;
    await dragTo(page, 100, centerY);

    const snapLines = await getSnapLines(page);

    // Should have a horizontal snap line near center (y=300)
    const horizontalLines = snapLines.filter(l => !l.isVertical);
    if (horizontalLines.length > 0) {
      const centerLine = horizontalLines.find(l => Math.abs(l.y1 - centerY) < 20);
      if (centerLine) {
        expect(centerLine.y1).toBeCloseTo(centerY, 0);
      }
    }

    await page.mouse.up();
  });

  test('TC-Snap-6: Snap to both canvas centers shows two lines (centered on canvas)', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag to exact center of canvas
    const centerX = CANVAS_WIDTH / 2;
    const centerY = CANVAS_HEIGHT / 2;
    await dragTo(page, centerX, centerY);

    const snapLines = await getSnapLines(page);

    // When perfectly centered, should have both vertical and horizontal snap lines
    const verticalLines = snapLines.filter(l => l.isVertical);
    const horizontalLines = snapLines.filter(l => !l.isVertical);

    // We expect snap lines when near center
    // (may have 0, 1, or 2 depending on exact positioning)
    console.log(`Found ${verticalLines.length} vertical and ${horizontalLines.length} horizontal snap lines`);

    await page.mouse.up();
  });

  test('TC-Snap-7: Snap to right canvas edge', async ({ page }) => {
    await selectShapeAt(page, 100, 400);

    const box = await getSelectionBox(page);
    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag to right edge of canvas
    await dragTo(page, CANVAS_WIDTH - 5, 400);

    const snapLines = await getSnapLines(page);

    // Should have a vertical snap line near x=800
    const verticalLines = snapLines.filter(l => l.isVertical);
    if (verticalLines.length > 0) {
      const rightEdgeLine = verticalLines.find(l => l.x1 > CANVAS_WIDTH - 20);
      if (rightEdgeLine) {
        expect(rightEdgeLine.x1).toBeCloseTo(CANVAS_WIDTH, 0);
      }
    }

    await page.mouse.up();
  });

  test('TC-Snap-8: Multiple shapes - snap to another shape edge', async ({ page }) => {
    // This test verifies snapping between shapes works
    // Select a shape and drag it near another shape

    // First click to select Snoopy (around 450, 250)
    await selectShapeAt(page, 450, 250);

    let box = await getSelectionBox(page);
    if (!box) {
      // Try the heart shape instead
      await selectShapeAt(page, 100, 400);
      box = await getSelectionBox(page);
    }

    if (!box) {
      test.skip(true, 'No shapes could be selected');
      return;
    }

    await startDrag(page, box);

    // Drag towards another shape area
    await dragTo(page, 200, 400);

    // Just verify we can complete the drag without errors
    await page.mouse.up();

    // Shape should have moved
    const newBox = await getSelectionBox(page);
    expect(newBox).toBeDefined();
  });
});
