import { chromium } from 'playwright';

const CHROMIUM_PATH = '/root/.cache/ms-playwright/chromium-1194/chrome-linux/chrome';

async function sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
}

async function main() {
    console.log('Launching browser...');
    const browser = await chromium.launch({
        executablePath: CHROMIUM_PATH,
        headless: true
    });

    const context = await browser.newContext({
        viewport: { width: 1280, height: 720 },
        recordVideo: {
            dir: '/home/user/canvas-rs/screenshots',
            size: { width: 1280, height: 720 }
        }
    });

    const page = await context.newPage();

    try {
        console.log('Navigating to app...');
        await page.goto('http://localhost:8090', { waitUntil: 'networkidle', timeout: 30000 });
        await sleep(2000);

        // Take initial screenshot
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_01_initial.png' });
        console.log('Screenshot 1: Initial state');

        // === PART 1: Selection and Translation ===

        // Click on a Snoopy shape (center-right area)
        console.log('Clicking on Snoopy shape...');
        await page.mouse.click(700, 350);
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_02_selected.png' });
        console.log('Screenshot 2: Shape selected');

        // Drag to translate
        console.log('Dragging to translate...');
        await page.mouse.move(700, 350);
        await page.mouse.down();
        await sleep(200);

        for (let i = 0; i <= 15; i++) {
            await page.mouse.move(700 - i * 4, 350 - i * 2);
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(800);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_03_translated.png' });
        console.log('Screenshot 3: Shape translated');

        // === PART 2: Scaling via corner handle ===

        // Click on a triangle to select it (easier to see scaling)
        console.log('Selecting triangle for scaling...');
        await page.mouse.click(500, 290);  // Click on one of the triangles
        await sleep(800);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_04_triangle_selected.png' });
        console.log('Screenshot 4: Triangle selected for scaling');

        // The triangle's bounding box should have handles
        // Drag the bottom-right corner to scale it larger
        // Triangle is small, so handles are close to (500, 290)
        // Let's estimate bottom-right handle position
        console.log('Scaling via bottom-right handle...');
        await page.mouse.move(520, 310);  // Move to approximate bottom-right handle
        await page.mouse.down();
        await sleep(200);

        // Drag outward to scale up
        for (let i = 0; i <= 15; i++) {
            await page.mouse.move(520 + i * 3, 310 + i * 3);
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(800);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_05_scaled.png' });
        console.log('Screenshot 5: Shape scaled');

        // === PART 3: Flipping via handle ===

        // Select the shape again (it may have been deselected)
        console.log('Selecting for flip...');
        await page.mouse.click(550, 340);  // Click on scaled triangle
        await sleep(800);

        // Drag the right handle past the left edge to flip horizontally
        // First find where the right handle is (approximately right edge of selection)
        console.log('Flipping horizontally via right handle...');
        await page.mouse.move(590, 340);  // Approximate right handle position
        await page.mouse.down();
        await sleep(200);

        // Drag past the left edge to flip
        for (let i = 0; i <= 25; i++) {
            await page.mouse.move(590 - i * 8, 340);  // Move left, eventually past original left edge
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(800);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_06_flipped_x.png' });
        console.log('Screenshot 6: Flipped horizontally');

        // === PART 4: Marquee selection ===

        // Deselect first
        await page.mouse.click(100, 100);
        await sleep(500);

        // Marquee selection to select multiple shapes
        console.log('Marquee selection...');
        await page.mouse.move(450, 200);
        await page.mouse.down();
        await sleep(200);

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(450 + i * 20, 200 + i * 15);
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_07_marquee.png' });
        console.log('Screenshot 7: Multiple shapes selected');

        // Translate the group
        console.log('Translating group...');
        await page.mouse.move(650, 350);
        await page.mouse.down();
        await sleep(200);

        for (let i = 0; i <= 15; i++) {
            await page.mouse.move(650 + i * 3, 350);
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(800);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_08_group_translated.png' });
        console.log('Screenshot 8: Group translated');

        // === PART 5: Flip the group vertically ===

        // Find bottom handle of the group selection and drag it past top
        console.log('Flipping group vertically...');
        // The group's bottom handle should be somewhere around y=500
        await page.mouse.move(720, 480);  // Approximate bottom handle
        await page.mouse.down();
        await sleep(200);

        // Drag up past the top to flip
        for (let i = 0; i <= 30; i++) {
            await page.mouse.move(720, 480 - i * 10);  // Move up
            await sleep(40);
        }

        await page.mouse.up();
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_09_group_flipped.png' });
        console.log('Screenshot 9: Group flipped vertically');

        await sleep(500);

    } catch (error) {
        console.error('Error during recording:', error);
    }

    // Get video path
    const videoPath = await page.video().path();
    console.log('Video recorded at:', videoPath);

    await page.close();
    await context.close();
    await browser.close();

    console.log('Done! Video saved.');
}

main().catch(console.error);
