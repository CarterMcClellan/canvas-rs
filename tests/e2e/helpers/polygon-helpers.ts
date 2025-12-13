import { Locator } from '@playwright/test';

export interface Point {
  x: number;
  y: number;
}

export function parsePolygonPoints(pointsString: string): Point[] {
  return pointsString.split(' ').map(pair => {
    const [x, y] = pair.split(',').map(Number);
    return { x, y };
  });
}

export function stringifyPolygonPoints(points: Point[]): string {
  return points.map(p => `${p.x},${p.y}`).join(' ');
}

export async function getPolygonPoints(polygon: Locator): Promise<Point[]> {
  const pointsString = await polygon.getAttribute('points');
  if (!pointsString) throw new Error('Polygon points not found');
  return parsePolygonPoints(pointsString);
}

export function calculateBoundingBox(points: Point[]): {
  x: number;
  y: number;
  width: number;
  height: number;
} {
  const xs = points.map(p => p.x);
  const ys = points.map(p => p.y);
  const minX = Math.min(...xs);
  const minY = Math.min(...ys);
  const maxX = Math.max(...xs);
  const maxY = Math.max(...ys);

  return {
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  };
}

export function pointsEqual(p1: Point, p2: Point, tolerance: number = 1): boolean {
  return Math.abs(p1.x - p2.x) < tolerance && Math.abs(p1.y - p2.y) < tolerance;
}
