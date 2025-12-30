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

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(700 - i * 5, 350 - i * 3);
            await sleep(50);
        }

        await page.mouse.up();
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_03_translated.png' });
        console.log('Screenshot 3: Shape translated');

        // Deselect
        await page.mouse.click(100, 100);
        await sleep(500);

        // Marquee selection
        console.log('Marquee selection...');
        await page.mouse.move(500, 200);
        await page.mouse.down();
        await sleep(200);

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(500 + i * 15, 200 + i * 15);
            await sleep(50);
        }

        await page.mouse.up();
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_04_marquee.png' });
        console.log('Screenshot 4: Multiple shapes selected');

        // Translate all selected
        console.log('Translating multiple shapes...');
        await page.mouse.move(650, 350);
        await page.mouse.down();
        await sleep(200);

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(650 + i * 4, 350);
            await sleep(50);
        }

        await page.mouse.up();
        await sleep(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/fresh_05_final.png' });
        console.log('Screenshot 5: Final state');

        await sleep(1000);

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
