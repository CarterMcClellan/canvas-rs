import { Point } from '../helpers/polygon-helpers';

export const INITIAL_POLYGONS: Point[][] = [
  [
    { x: 230, y: 220 },
    { x: 260, y: 220 },
    { x: 245, y: 250 },
  ],
  [
    { x: 270, y: 230 },
    { x: 300, y: 230 },
    { x: 285, y: 260 },
  ],
  [
    { x: 240, y: 270 },
    { x: 270, y: 270 },
    { x: 255, y: 300 },
  ],
];

export const INITIAL_BOUNDING_BOX = {
  x: 230,
  y: 220,
  width: 70,
  height: 80,
};
