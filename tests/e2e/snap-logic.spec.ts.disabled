import { test, expect } from '@playwright/test';
import { calculateSnap } from '../src/components/ResizableCanvas/snapLogic';
import { BoundingBox, Polygon } from '../src/components/ResizableCanvas/types';

const mockPolygons: Polygon[] = [
  { points: "0,0 100,0 100,100 0,100", fill: "none", stroke: "black", strokeWidth: 1 },
  { points: "200,0 300,0 300,100 200,100", fill: "none", stroke: "black", strokeWidth: 1 },
];

const canvasWidth = 800;
const canvasHeight = 600;
const threshold = 5;

test.describe('Snap Logic Unit Tests', () => {
  test('should return no snap when far from any target', () => {
    // Use position that doesn't accidentally align with polygon centers (which are at y=50)
    const proposedBox: BoundingBox = { x: 150, y: 150, width: 10, height: 10 };
    const result = calculateSnap(proposedBox, mockPolygons, [], canvasWidth, canvasHeight, threshold);

    expect(result.translation.x).toBe(0);
    expect(result.translation.y).toBe(0);
    expect(result.guidelines).toHaveLength(0);
  });

  test('should snap to left edge of a polygon', () => {
    // Target polygon starts at x=200.
    // Proposed box right edge at 198. width 10 -> x = 188.
    // Wait, logic matches:
    // Box Start (x), Center (x+w/2), End (x+w)
    // Target Start, Center, End

    // Let's try snap start-to-start. Target at 200. Proposed at 202.
    // Use y position that doesn't align with polygon edges (y=150 avoids 0, 50, 100)
    const proposedBox: BoundingBox = { x: 202, y: 150, width: 50, height: 50 };
    // dist = 2. < 5. Snap delta = 200 - 202 = -2.

    const result = calculateSnap(proposedBox, mockPolygons, [], canvasWidth, canvasHeight, threshold);

    expect(result.translation.x).toBe(-2);
    expect(result.translation.y).toBe(0);
    expect(result.guidelines).toHaveLength(1);
    expect(result.guidelines[0].type).toBe('vertical');
    expect(result.guidelines[0].pos).toBe(200);
  });

  test('should snap to top edge of a polygon', () => {
    // Target polygon y=0. Proposed y=3.
    // Use x position and width that don't align with polygon edges
    // Polygon edges are at x=[0,50,100] and x=[200,250,300]
    // Using x=140, width=40 gives edges at [140,160,180] - no alignment
    const proposedBox: BoundingBox = { x: 140, y: 3, width: 40, height: 50 };
    const result = calculateSnap(proposedBox, mockPolygons, [], canvasWidth, canvasHeight, threshold);

    expect(result.translation.x).toBe(0);
    expect(result.translation.y).toBe(-3);
    expect(result.guidelines).toHaveLength(1);
    expect(result.guidelines[0].type).toBe('horizontal');
    expect(result.guidelines[0].pos).toBe(0);
  });

  test('should prioritize closest snap', () => {
     // Two targets: 0 and 200.
     // Proposed x = 4. Closer to 0 (dist 4) than 200.
     // Proposed x = 10. Outside threshold for 0.
     
     // Let's add a 3rd polygon at 105
     const polygonsWithClose = [
         ...mockPolygons,
         { points: "105,0 115,0 115,10 105,10", fill: "none", stroke: "black", strokeWidth: 1 }
     ];
     
     // Polygon 1 ends at 100. Polygon 3 starts at 105.
     // Proposed box starts at 102.
     // Dist to 100 is 2. Dist to 105 is 3.
     // Should snap to 100.
     
     const proposedBox: BoundingBox = { x: 102, y: 200, width: 10, height: 10 };
     const result = calculateSnap(proposedBox, polygonsWithClose, [], canvasWidth, canvasHeight, threshold);

     expect(result.translation.x).toBe(-2); // Snap to 100
     expect(result.guidelines[0].pos).toBe(100);
  });

  test('should respect excluded IDs (self-snap avoidance)', () => {
      // If we are moving polygon index 0, it shouldn't snap to itself.
      const proposedBox: BoundingBox = { x: 2, y: 2, width: 100, height: 100 }; // Close to 0,0
      // If we include it, it snaps to 0.
      // If we exclude index 0, next closest is 200 (far) or canvas edges (0).
      // Canvas edge is at 0. So it will still snap to 0 but because of canvas, not polygon 0.
      
      // Let's use a polygon in middle.
      const middlePolys = [
          { points: "0,0 10,0 10,10 0,10", fill: "", stroke: "", strokeWidth: 0 }, // idx 0
          { points: "100,100 110,100 110,110 100,110", fill: "", stroke: "", strokeWidth: 0 } // idx 1
      ];
      
      // We move idx 1. Proposed at 102,102. 
      // If we exclude idx 1, it should look at idx 0 (0,0) - too far.
      // Canvas edges: 0,0. too far.
      
      const proposedBox2: BoundingBox = { x: 102, y: 102, width: 10, height: 10 };
      const result = calculateSnap(proposedBox2, middlePolys, [1], canvasWidth, canvasHeight, threshold);
      
      // Should be no snap
      expect(result.translation.x).toBe(0);
      expect(result.translation.y).toBe(0);
  });
});
