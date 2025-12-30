import { chromium } from 'playwright';
import { spawn } from 'child_process';
import { setTimeout } from 'timers/promises';

const CHROMIUM_PATH = '/root/.cache/ms-playwright/chromium-1194/chrome-linux/chrome';

async function main() {
    console.log('Starting trunk server...');

    // Start trunk serve in background
    const trunk = spawn('trunk', ['serve', '--release', '--port', '8090'], {
        cwd: '/home/user/canvas-rs',
        stdio: ['ignore', 'pipe', 'pipe']
    });

    trunk.stdout.on('data', (data) => {
        console.log(`trunk: ${data}`);
    });

    trunk.stderr.on('data', (data) => {
        console.log(`trunk: ${data}`);
    });

    // Wait for server to be ready
    console.log('Waiting for server to start...');
    await setTimeout(30000); // Wait 30 seconds for WASM build

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
        await page.goto('http://localhost:8090', { waitUntil: 'networkidle', timeout: 60000 });
        await setTimeout(2000);

        // Take initial screenshot
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/01_initial.png' });
        console.log('Screenshot 1: Initial state');

        // Click on a Snoopy shape (approximately center-right area where Snoopy is)
        console.log('Clicking on Snoopy shape...');
        await page.mouse.click(700, 350);
        await setTimeout(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/02_selected.png' });
        console.log('Screenshot 2: Shape selected');

        // Drag to translate
        console.log('Dragging to translate...');
        await page.mouse.move(700, 350);
        await page.mouse.down();
        await setTimeout(200);

        // Slow drag for visibility
        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(700 - i * 5, 350 - i * 3);
            await setTimeout(50);
        }

        await page.mouse.up();
        await setTimeout(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/03_translated.png' });
        console.log('Screenshot 3: Shape translated');

        // Click elsewhere to deselect
        await page.mouse.click(100, 100);
        await setTimeout(500);

        // Marquee selection - drag from top-left to bottom-right
        console.log('Marquee selection...');
        await page.mouse.move(500, 200);
        await page.mouse.down();
        await setTimeout(200);

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(500 + i * 15, 200 + i * 15);
            await setTimeout(50);
        }

        await page.mouse.up();
        await setTimeout(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/04_marquee.png' });
        console.log('Screenshot 4: Multiple shapes selected');

        // Translate all selected
        console.log('Translating multiple shapes...');
        await page.mouse.move(650, 350);
        await page.mouse.down();
        await setTimeout(200);

        for (let i = 0; i <= 20; i++) {
            await page.mouse.move(650 + i * 4, 350);
            await setTimeout(50);
        }

        await page.mouse.up();
        await setTimeout(1000);
        await page.screenshot({ path: '/home/user/canvas-rs/screenshots/05_final.png' });
        console.log('Screenshot 5: Final state');

        await setTimeout(1000);

    } catch (error) {
        console.error('Error during recording:', error);
    }

    await page.close();
    await context.close();
    await browser.close();

    console.log('Stopping trunk server...');
    trunk.kill();

    console.log('Done! Check screenshots/ for video and images.');
}

main().catch(console.error);
