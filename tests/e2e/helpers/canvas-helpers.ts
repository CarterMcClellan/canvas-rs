import { Page, Locator, expect } from '@playwright/test';

export async function getSVGOffset(page: Page): Promise<{ x: number; y: number }> {
  const svg = page.locator('[data-testid="main-canvas"]');
  const box = await svg.boundingBox();
  if (!box) throw new Error('SVG element not found');
  return { x: box.x, y: box.y };
}

export async function getElementCenter(element: Locator): Promise<{ x: number; y: number }> {
  const box = await element.boundingBox();
  if (!box) throw new Error('Element not found');
  return {
    x: box.x + box.width / 2,
    y: box.y + box.height / 2,
  };
}

export async function dragFromTo(
  page: Page,
  fromX: number,
  fromY: number,
  toX: number,
  toY: number,
  steps: number = 10
): Promise<void> {
  await page.mouse.move(fromX, fromY);
  await page.mouse.down();

  for (let i = 1; i <= steps; i++) {
    const x = fromX + ((toX - fromX) * i) / steps;
    const y = fromY + ((toY - fromY) * i) / steps;
    await page.mouse.move(x, y);
    await page.waitForTimeout(10);
  }

  await page.mouse.up();
  await page.waitForTimeout(100);
}

export async function drawSelectionRectangle(
  page: Page,
  x1: number,
  y1: number,
  x2: number,
  y2: number
): Promise<void> {
  const offset = await getSVGOffset(page);

  await dragFromTo(
    page,
    offset.x + x1,
    offset.y + y1,
    offset.x + x2,
    offset.y + y2
  );
}

export async function dragHandle(
  page: Page,
  handleType: 'left' | 'right' | 'top' | 'bottom' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right',
  deltaX: number,
  deltaY: number
): Promise<void> {
  // Use test-id to locate the handle directly
  const handle = page.locator(`[data-testid="resize-handle-${handleType}"]`);
  const handleBox = await handle.boundingBox();
  if (!handleBox) throw new Error(`Handle ${handleType} not found`);

  const handleX = handleBox.x + handleBox.width / 2;
  const handleY = handleBox.y + handleBox.height / 2;

  await dragFromTo(page, handleX, handleY, handleX + deltaX, handleY + deltaY);
}

export async function startDragHandle(
  page: Page,
  handleType: 'left' | 'right' | 'top' | 'bottom' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right',
  deltaX: number,
  deltaY: number,
  steps: number = 20
): Promise<void> {
  // Ensure a selection exists; if not, draw a marquee around all polygons (test helper resilience).
  const selectionBox = page.locator('[data-testid="selection-bounding-box"]');
  if (await selectionBox.count() === 0) {
    // Try helper button first (fast path)
    await page.evaluate(() => {
      const btn = document.querySelector('[data-testid="select-all-helper"]') as HTMLElement | null;
      if (btn) btn.click();
    });
    await page.waitForTimeout(50);

    await drawSelectionRectangle(page, 220, 210, 310, 310);
    await page.waitForTimeout(50);
  }

  const handle = page.locator(`[data-testid="resize-handle-${handleType}"]`);
  await handle.waitFor({ state: 'visible', timeout: 5000 });
  const handleBox = await handle.boundingBox();
  if (!handleBox) throw new Error(`Handle ${handleType} not found`);

  const fromX = handleBox.x + handleBox.width / 2;
  const fromY = handleBox.y + handleBox.height / 2;
  const toX = fromX + deltaX;
  const toY = fromY + deltaY;

  await page.mouse.move(fromX, fromY);
  await page.mouse.down();

  for (let i = 1; i <= steps; i++) {
    const x = fromX + ((toX - fromX) * i) / steps;
    const y = fromY + ((toY - fromY) * i) / steps;
    await page.mouse.move(x, y);
    await page.waitForTimeout(10);
  }

  // Allow Yew to apply state updates triggered during the drag before assertions
  await page.waitForTimeout(50);
}

export async function releaseMouse(page: Page): Promise<void> {
  await page.mouse.up();
  await page.waitForTimeout(100);
}

export async function waitForSVGReady(page: Page): Promise<void> {
  const svg = page.locator('[data-testid="main-canvas"]');
  await expect(svg).toBeVisible();
  await expect(svg.locator('polygon')).toHaveCount(3);
  await page.waitForTimeout(300);
}

export async function getFixedAnchorPosition(page: Page): Promise<{ x: number; y: number }> {
  const fixedAnchor = page.locator('[data-is-fixed-anchor="true"]');
  const box = await fixedAnchor.boundingBox();
  if (!box) throw new Error('Fixed anchor not found');
  return {
    x: box.x + box.width / 2,
    y: box.y + box.height / 2,
  };
}
