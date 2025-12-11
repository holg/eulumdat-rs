/**
 * 3D Butterfly Diagram View
 *
 * Interactive 3D visualization of photometric data with touch rotation
 */

import { EulumdatEngine, LuminaireInfo, SvgTheme } from '../model/EulumdatEngine';

/**
 * 3D Point with rotation and projection methods
 */
class Point3D {
  x: number;
  y: number;
  z: number;

  constructor(x: number, y: number, z: number) {
    this.x = x;
    this.y = y;
    this.z = z;
  }

  rotateX(angle: number): Point3D {
    const cosA = Math.cos(angle);
    const sinA = Math.sin(angle);
    return new Point3D(
      this.x,
      this.y * cosA - this.z * sinA,
      this.y * sinA + this.z * cosA
    );
  }

  rotateY(angle: number): Point3D {
    const cosA = Math.cos(angle);
    const sinA = Math.sin(angle);
    return new Point3D(
      this.x * cosA + this.z * sinA,
      this.y,
      -this.x * sinA + this.z * cosA
    );
  }

  project(cx: number, cy: number, scale: number): number[] {
    const perspective = 600.0;
    const zOffset = 300.0;
    const factor = perspective / (perspective + this.z + zOffset);
    return [
      cx + this.x * scale * factor,
      cy - this.y * scale * factor
    ];
  }
}

/**
 * Wing data for 3D rendering
 */
class Wing {
  cAngle: number;
  points: Point3D[];
  colorHue: number;

  constructor(cAngle: number, points: Point3D[], colorHue: number) {
    this.cAngle = cAngle;
    this.points = points;
    this.colorHue = colorHue;
  }
}

/**
 * Helper class for sorting wings by depth
 */
class WingWithDepth {
  wing: Wing;
  avgZ: number;

  constructor(wing: Wing, avgZ: number) {
    this.wing = wing;
    this.avgZ = avgZ;
  }
}

@Component
export struct Butterfly3DView {
  @Prop engine: EulumdatEngine;
  @Prop info: LuminaireInfo;
  @Prop isDarkMode: boolean;

  @State private rotationX: number = 0.5;
  @State private rotationY: number = 0.0;
  @State private autoRotate: boolean = true;
  @State private isDragging: boolean = false;
  @State private lastTouchX: number = 0;
  @State private lastTouchY: number = 0;
  private animationId: number = 0;
  private canvasSettings: RenderingContextSettings = new RenderingContextSettings(true);
  private context: CanvasRenderingContext2D = new CanvasRenderingContext2D(this.canvasSettings);

  aboutToAppear(): void {
    this.startAutoRotation();
  }

  aboutToDisappear(): void {
    this.stopAutoRotation();
  }

  private startAutoRotation(): void {
    if (this.autoRotate && !this.isDragging) {
      this.animationId = setInterval(() => {
        if (this.autoRotate && !this.isDragging) {
          this.rotationY += 0.02;
        }
      }, 16);
    }
  }

  private stopAutoRotation(): void {
    if (this.animationId !== 0) {
      clearInterval(this.animationId);
      this.animationId = 0;
    }
  }

  private toggleAutoRotate(): void {
    this.autoRotate = !this.autoRotate;
    if (this.autoRotate) {
      this.startAutoRotation();
    } else {
      this.stopAutoRotation();
    }
  }

  private resetView(): void {
    this.rotationX = 0.5;
    this.rotationY = 0.0;
  }

  /**
   * Build wing geometry from sampled intensity data
   */
  private buildWings(): Wing[] {
    const wings: Wing[] = [];
    const numCPlanes = 12;
    const numGAngles = 19; // 0 to 180 in 10-degree steps

    for (let c = 0; c < numCPlanes; c++) {
      const cAngle = c * 30;
      const cRad = cAngle * Math.PI / 180;
      const points: Point3D[] = [];

      // Start at center
      points.push(new Point3D(0, 0, 0));

      // Sample intensities along gamma angles
      for (let g = 0; g <= numGAngles; g++) {
        const gAngle = g * 10;
        const gRad = gAngle * Math.PI / 180;

        // Sample intensity and normalize
        const intensity = this.engine.sampleIntensity(cAngle, gAngle);
        const maxIntensity = this.info.maxIntensity > 0 ? this.info.maxIntensity : 1;
        const r = intensity / maxIntensity;

        // Convert spherical to Cartesian
        const x = r * Math.sin(gRad) * Math.cos(cRad);
        const y = r * Math.sin(gRad) * Math.sin(cRad);
        const z = r * Math.cos(gRad);

        points.push(new Point3D(x, y, -z));
      }

      const colorHue = ((cAngle / 360) * 240 + 180) % 360;
      wings.push(new Wing(cAngle, points, colorHue));
    }

    return wings;
  }

  /**
   * Convert HSL to RGB hex color
   */
  private hslToHex(h: number, s: number, l: number, alpha: number = 1): string {
    const c = (1 - Math.abs(2 * l - 1)) * s;
    const x = c * (1 - Math.abs((h / 60) % 2 - 1));
    const m = l - c / 2;

    let r = 0, g = 0, b = 0;
    if (h < 60) { r = c; g = x; b = 0; }
    else if (h < 120) { r = x; g = c; b = 0; }
    else if (h < 180) { r = 0; g = c; b = x; }
    else if (h < 240) { r = 0; g = x; b = c; }
    else if (h < 300) { r = x; g = 0; b = c; }
    else { r = c; g = 0; b = x; }

    const toHex = (v: number): string => {
      const hex = Math.round((v + m) * 255).toString(16);
      return hex.length === 1 ? '0' + hex : hex;
    };

    if (alpha < 1) {
      const alphaHex = Math.round(alpha * 255).toString(16);
      return `#${toHex(r)}${toHex(g)}${toHex(b)}${alphaHex.length === 1 ? '0' + alphaHex : alphaHex}`;
    }
    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  }

  /**
   * Draw the 3D scene on canvas
   */
  private drawScene(): void {
    const ctx = this.context;
    const width = ctx.width;
    const height = ctx.height;
    const cx = width / 2;
    const cy = height / 2;
    const scale = Math.min(width, height) / 2 * 0.7;

    // Clear canvas
    const bgColor = this.isDarkMode ? '#1a1a2e' : '#ffffff';
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    const gridColor = this.isDarkMode ? '#404060' : '#e0e0e0';

    // Draw grid
    this.drawGrid(ctx, cx, cy, scale, gridColor);

    // Build and sort wings by depth
    const wings = this.buildWings();
    const wingsWithDepth: WingWithDepth[] = [];
    for (const wing of wings) {
      let avgZ = 0;
      for (const p of wing.points) {
        const rotated = p.rotateX(this.rotationX).rotateY(this.rotationY);
        avgZ += rotated.z;
      }
      avgZ /= wing.points.length;
      wingsWithDepth.push(new WingWithDepth(wing, avgZ));
    }
    wingsWithDepth.sort((a: WingWithDepth, b: WingWithDepth) => b.avgZ - a.avgZ);
    const sortedWings: Wing[] = wingsWithDepth.map((item: WingWithDepth) => item.wing);

    // Draw wings
    for (const wing of sortedWings) {
      this.drawWing(ctx, wing, cx, cy, scale);
    }

    // Draw center point
    const centerColor = this.isDarkMode ? '#ffffff' : '#333333';
    ctx.fillStyle = centerColor;
    ctx.beginPath();
    ctx.arc(cx, cy, 4, 0, Math.PI * 2);
    ctx.fill();
  }

  /**
   * Draw grid circles and C-plane lines
   */
  private drawGrid(ctx: CanvasRenderingContext2D, cx: number, cy: number, scale: number, gridColor: string): void {
    ctx.strokeStyle = gridColor;
    ctx.lineWidth = 1;

    // Draw concentric circles
    for (let i = 1; i <= 4; i++) {
      const r = i / 4;
      ctx.beginPath();
      for (let j = 0; j <= 36; j++) {
        const cAngle = j * 10;
        const cRad = cAngle * Math.PI / 180;

        const point = new Point3D(r * Math.cos(cRad), r * Math.sin(cRad), 0)
          .rotateX(this.rotationX)
          .rotateY(this.rotationY);
        const projected = point.project(cx, cy, scale);

        if (j === 0) {
          ctx.moveTo(projected[0], projected[1]);
        } else {
          ctx.lineTo(projected[0], projected[1]);
        }
      }
      ctx.closePath();
      ctx.stroke();
    }

    // Draw C-plane direction lines
    for (let i = 0; i < 8; i++) {
      const cAngle = i * 45;
      const cRad = cAngle * Math.PI / 180;

      const p1 = new Point3D(0, 0, 0)
        .rotateX(this.rotationX)
        .rotateY(this.rotationY)
        .project(cx, cy, scale);

      const p2 = new Point3D(Math.cos(cRad), Math.sin(cRad), 0)
        .rotateX(this.rotationX)
        .rotateY(this.rotationY)
        .project(cx, cy, scale);

      ctx.beginPath();
      ctx.moveTo(p1[0], p1[1]);
      ctx.lineTo(p2[0], p2[1]);
      ctx.stroke();
    }
  }

  /**
   * Draw a single wing
   */
  private drawWing(ctx: CanvasRenderingContext2D, wing: Wing, cx: number, cy: number, scale: number): void {
    if (wing.points.length < 2) return;

    ctx.beginPath();
    let first = true;

    for (const point of wing.points) {
      const rotated = point.rotateX(this.rotationX).rotateY(this.rotationY);
      const projected = rotated.project(cx, cy, scale);

      if (first) {
        ctx.moveTo(projected[0], projected[1]);
        first = false;
      } else {
        ctx.lineTo(projected[0], projected[1]);
      }
    }
    ctx.closePath();

    // Fill with semi-transparent color
    ctx.fillStyle = this.hslToHex(wing.colorHue, 0.6, 0.5, 0.5);
    ctx.fill();

    // Stroke with brighter color
    ctx.strokeStyle = this.hslToHex(wing.colorHue, 0.7, 0.6);
    ctx.lineWidth = 1.5;
    ctx.stroke();
  }

  build() {
    Column() {
      // 3D Canvas
      Canvas(this.context)
        .width('100%')
        .layoutWeight(1)
        .backgroundColor(this.isDarkMode ? '#1a1a2e' : '#ffffff')
        .onReady(() => {
          this.drawScene();
        })
        .onTouch((event: TouchEvent) => {
          if (event.type === TouchType.Down) {
            this.isDragging = true;
            this.lastTouchX = event.touches[0].x;
            this.lastTouchY = event.touches[0].y;
          } else if (event.type === TouchType.Move && this.isDragging) {
            const dx = event.touches[0].x - this.lastTouchX;
            const dy = event.touches[0].y - this.lastTouchY;
            this.rotationY += dx * 0.005;
            this.rotationX += dy * 0.005;
            this.rotationX = Math.max(-1.5, Math.min(1.5, this.rotationX));
            this.lastTouchX = event.touches[0].x;
            this.lastTouchY = event.touches[0].y;
            this.drawScene();
          } else if (event.type === TouchType.Up || event.type === TouchType.Cancel) {
            this.isDragging = false;
          }
        })

      // Labels
      Row() {
        Text('3D Photometric Solid')
          .fontSize(12)
          .fontColor(this.isDarkMode ? '#a0a0a0' : '#666666')

        Blank()

        Text('Drag to rotate')
          .fontSize(10)
          .fontColor(this.isDarkMode ? '#707070' : '#999999')
      }
      .width('100%')
      .padding(8)

      // Controls
      Row() {
        Button(this.autoRotate ? 'Pause' : 'Auto')
          .onClick(() => this.toggleAutoRotate())
          .backgroundColor(this.autoRotate ? $r('app.color.primary') : $r('app.color.text_secondary'))
          .fontColor(Color.White)
          .margin({ right: 8 })

        Button('Reset')
          .onClick(() => this.resetView())
          .backgroundColor(Color.Transparent)
          .fontColor($r('app.color.primary'))
          .borderWidth(1)
          .borderColor($r('app.color.primary'))
      }
      .width('100%')
      .justifyContent(FlexAlign.Center)
      .padding(8)
      .backgroundColor($r('app.color.card_background'))
    }
    .width('100%')
    .height('100%')
  }
}
