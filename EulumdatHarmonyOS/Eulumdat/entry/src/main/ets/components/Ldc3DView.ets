/**
 * LDC 3D View - Photometric Solid Visualization
 *
 * Renders the 3D photometric solid (LDC - Luminous Distribution Curve)
 * using mesh data generated from the Rust library.
 *
 * Uses Canvas rendering with proper triangle mesh and lighting simulation.
 * Structured for future upgrade to ArkGraphics 3D when direct mesh creation APIs are available.
 */

import { EulumdatEngine, LuminaireInfo, LdcMeshData, ColorMode } from '../model/EulumdatEngine';

/**
 * 3D Vector for calculations
 */
class Vec3 {
  x: number;
  y: number;
  z: number;

  constructor(x: number = 0, y: number = 0, z: number = 0) {
    this.x = x;
    this.y = y;
    this.z = z;
  }

  static fromArray(arr: Float32Array, index: number): Vec3 {
    return new Vec3(arr[index], arr[index + 1], arr[index + 2]);
  }

  add(v: Vec3): Vec3 {
    return new Vec3(this.x + v.x, this.y + v.y, this.z + v.z);
  }

  sub(v: Vec3): Vec3 {
    return new Vec3(this.x - v.x, this.y - v.y, this.z - v.z);
  }

  scale(s: number): Vec3 {
    return new Vec3(this.x * s, this.y * s, this.z * s);
  }

  dot(v: Vec3): number {
    return this.x * v.x + this.y * v.y + this.z * v.z;
  }

  cross(v: Vec3): Vec3 {
    return new Vec3(
      this.y * v.z - this.z * v.y,
      this.z * v.x - this.x * v.z,
      this.x * v.y - this.y * v.x
    );
  }

  normalize(): Vec3 {
    const len = Math.sqrt(this.x * this.x + this.y * this.y + this.z * this.z);
    if (len < 0.0001) return new Vec3(0, 1, 0);
    return new Vec3(this.x / len, this.y / len, this.z / len);
  }

  rotateX(angle: number): Vec3 {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return new Vec3(
      this.x,
      this.y * cos - this.z * sin,
      this.y * sin + this.z * cos
    );
  }

  rotateY(angle: number): Vec3 {
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    return new Vec3(
      this.x * cos + this.z * sin,
      this.y,
      -this.x * sin + this.z * cos
    );
  }

  project(cx: number, cy: number, scale: number): [number, number] {
    const perspective = 500.0;
    const zOffset = 200.0;
    const factor = perspective / (perspective + this.z + zOffset);
    return [
      cx + this.x * scale * factor,
      cy - this.y * scale * factor
    ];
  }
}

/**
 * Triangle with depth for sorting
 */
interface Triangle {
  i0: number;
  i1: number;
  i2: number;
  depth: number;
}

@Component
export struct Ldc3DView {
  @Prop engine: EulumdatEngine;
  @Prop info: LuminaireInfo;
  @Prop isDarkMode: boolean;

  @State private rotationX: number = 0.4;
  @State private rotationY: number = 0.0;
  @State private autoRotate: boolean = true;
  @State private isDragging: boolean = false;
  @State private lastTouchX: number = 0;
  @State private lastTouchY: number = 0;
  @State private colorMode: ColorMode = ColorMode.Heatmap;
  @State private showWireframe: boolean = false;

  private animationId: number = 0;
  private canvasSettings: RenderingContextSettings = new RenderingContextSettings(true);
  private context: CanvasRenderingContext2D = new CanvasRenderingContext2D(this.canvasSettings);
  private meshData: LdcMeshData | null = null;

  aboutToAppear(): void {
    this.loadMesh();
    this.startAutoRotation();
  }

  aboutToDisappear(): void {
    this.stopAutoRotation();
  }

  private loadMesh(): void {
    try {
      // Generate mesh with current color mode
      // Using 10 degree steps for good quality/performance balance
      this.meshData = this.engine.generateLdcMesh(10, 10, 1.0, this.colorMode);
    } catch (e) {
      console.error('Failed to generate mesh:', e);
      this.meshData = null;
    }
  }

  private startAutoRotation(): void {
    if (this.autoRotate && !this.isDragging) {
      this.animationId = setInterval(() => {
        if (this.autoRotate && !this.isDragging) {
          this.rotationY += 0.015;
          this.drawScene();
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

  private cycleColorMode(): void {
    this.colorMode = (this.colorMode + 1) % 3;
    this.loadMesh();
    this.drawScene();
  }

  private toggleWireframe(): void {
    this.showWireframe = !this.showWireframe;
    this.drawScene();
  }

  private resetView(): void {
    this.rotationX = 0.4;
    this.rotationY = 0.0;
    this.drawScene();
  }

  /**
   * Draw the 3D mesh scene
   */
  private drawScene(): void {
    const ctx = this.context;
    if (!ctx || ctx.width === 0 || ctx.height === 0) return;

    const width = ctx.width;
    const height = ctx.height;
    const cx = width / 2;
    const cy = height / 2;
    const scale = Math.min(width, height) / 2 * 0.65;

    // Clear canvas
    const bgColor = this.isDarkMode ? '#1a1a2e' : '#f5f5f5';
    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, width, height);

    // Draw axes
    this.drawAxes(ctx, cx, cy, scale);

    // Draw mesh if available
    if (this.meshData && this.meshData.vertexCount > 0) {
      this.drawMesh(ctx, cx, cy, scale);
    } else {
      // Fallback: draw simple placeholder
      ctx.fillStyle = this.isDarkMode ? '#666666' : '#999999';
      ctx.font = '14px sans-serif';
      ctx.textAlign = 'center';
      ctx.fillText('No mesh data', cx, cy);
    }
  }

  /**
   * Draw coordinate axes
   */
  private drawAxes(ctx: CanvasRenderingContext2D, cx: number, cy: number, scale: number): void {
    const axisLength = 0.3;
    const axisColors = ['#ff4444', '#44ff44', '#4444ff']; // X, Y, Z

    // X axis (red)
    const xEnd = new Vec3(axisLength, 0, 0).rotateX(this.rotationX).rotateY(this.rotationY);
    const xProj = xEnd.project(cx, cy, scale);
    ctx.strokeStyle = axisColors[0];
    ctx.lineWidth = 2;
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(xProj[0], xProj[1]);
    ctx.stroke();

    // Y axis (green)
    const yEnd = new Vec3(0, axisLength, 0).rotateX(this.rotationX).rotateY(this.rotationY);
    const yProj = yEnd.project(cx, cy, scale);
    ctx.strokeStyle = axisColors[1];
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(yProj[0], yProj[1]);
    ctx.stroke();

    // Z axis (blue)
    const zEnd = new Vec3(0, 0, axisLength).rotateX(this.rotationX).rotateY(this.rotationY);
    const zProj = zEnd.project(cx, cy, scale);
    ctx.strokeStyle = axisColors[2];
    ctx.beginPath();
    ctx.moveTo(cx, cy);
    ctx.lineTo(zProj[0], zProj[1]);
    ctx.stroke();
  }

  /**
   * Draw the mesh with proper depth sorting and shading
   */
  private drawMesh(ctx: CanvasRenderingContext2D, cx: number, cy: number, scale: number): void {
    const mesh = this.meshData!;
    const positions = mesh.positions;
    const colors = mesh.colors;
    const indices = mesh.indices;

    // Light direction (from upper-right-front)
    const lightDir = new Vec3(0.5, 0.7, 0.5).normalize();

    // Transform all vertices
    const transformedVerts: Vec3[] = [];
    for (let i = 0; i < mesh.vertexCount; i++) {
      const v = Vec3.fromArray(positions, i * 3);
      const rotated = v.rotateX(this.rotationX).rotateY(this.rotationY);
      transformedVerts.push(rotated);
    }

    // Build triangles with depth
    const triangles: Triangle[] = [];
    const numTriangles = mesh.indexCount / 3;
    for (let t = 0; t < numTriangles; t++) {
      const i0 = indices[t * 3];
      const i1 = indices[t * 3 + 1];
      const i2 = indices[t * 3 + 2];

      // Calculate triangle center depth
      const v0 = transformedVerts[i0];
      const v1 = transformedVerts[i1];
      const v2 = transformedVerts[i2];
      const depth = (v0.z + v1.z + v2.z) / 3;

      triangles.push({ i0, i1, i2, depth });
    }

    // Sort by depth (back to front)
    triangles.sort((a, b) => a.depth - b.depth);

    // Draw triangles
    for (const tri of triangles) {
      const v0 = transformedVerts[tri.i0];
      const v1 = transformedVerts[tri.i1];
      const v2 = transformedVerts[tri.i2];

      // Calculate face normal for backface culling and lighting
      const edge1 = v1.sub(v0);
      const edge2 = v2.sub(v0);
      const normal = edge1.cross(edge2).normalize();

      // Backface culling - skip triangles facing away
      if (normal.z < -0.1) continue;

      // Project vertices
      const p0 = v0.project(cx, cy, scale);
      const p1 = v1.project(cx, cy, scale);
      const p2 = v2.project(cx, cy, scale);

      // Get vertex colors (average for face)
      const c0 = this.getVertexColor(colors, tri.i0);
      const c1 = this.getVertexColor(colors, tri.i1);
      const c2 = this.getVertexColor(colors, tri.i2);

      // Average color
      const avgR = (c0[0] + c1[0] + c2[0]) / 3;
      const avgG = (c0[1] + c1[1] + c2[1]) / 3;
      const avgB = (c0[2] + c1[2] + c2[2]) / 3;
      const avgA = (c0[3] + c1[3] + c2[3]) / 3;

      // Simple diffuse lighting
      const diffuse = Math.max(0.2, normal.dot(lightDir));

      // Apply lighting to color
      const r = Math.floor(avgR * 255 * diffuse);
      const g = Math.floor(avgG * 255 * diffuse);
      const b = Math.floor(avgB * 255 * diffuse);
      const a = avgA;

      // Draw filled triangle
      ctx.beginPath();
      ctx.moveTo(p0[0], p0[1]);
      ctx.lineTo(p1[0], p1[1]);
      ctx.lineTo(p2[0], p2[1]);
      ctx.closePath();

      ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a})`;
      ctx.fill();

      // Wireframe overlay
      if (this.showWireframe) {
        ctx.strokeStyle = this.isDarkMode ? 'rgba(255,255,255,0.3)' : 'rgba(0,0,0,0.2)';
        ctx.lineWidth = 0.5;
        ctx.stroke();
      }
    }
  }

  /**
   * Get RGBA color for a vertex from colors array
   */
  private getVertexColor(colors: Float32Array, index: number): [number, number, number, number] {
    const i = index * 4;
    return [colors[i], colors[i + 1], colors[i + 2], colors[i + 3]];
  }

  /**
   * Get color mode name for display
   */
  private getColorModeName(): string {
    switch (this.colorMode) {
      case ColorMode.Heatmap:
        return 'Heatmap';
      case ColorMode.CPlaneRainbow:
        return 'C-Plane';
      case ColorMode.Solid:
        return 'Solid';
      default:
        return 'Heatmap';
    }
  }

  build() {
    Column() {
      // 3D Canvas
      Canvas(this.context)
        .width('100%')
        .layoutWeight(1)
        .backgroundColor(this.isDarkMode ? '#1a1a2e' : '#f5f5f5')
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

      // Info row
      Row() {
        Text('3D Photometric Solid (LDC)')
          .fontSize(12)
          .fontColor(this.isDarkMode ? '#a0a0a0' : '#666666')

        Blank()

        Text(`${this.meshData?.vertexCount ?? 0} vertices`)
          .fontSize(10)
          .fontColor(this.isDarkMode ? '#707070' : '#999999')
      }
      .width('100%')
      .padding({ left: 12, right: 12, top: 6, bottom: 6 })

      // Controls
      Row() {
        Button(this.autoRotate ? 'Pause' : 'Auto')
          .onClick(() => this.toggleAutoRotate())
          .height(32)
          .fontSize(12)
          .backgroundColor(this.autoRotate ? $r('app.color.primary') : Color.Gray)
          .fontColor(Color.White)

        Button(this.getColorModeName())
          .onClick(() => this.cycleColorMode())
          .height(32)
          .fontSize(12)
          .backgroundColor(Color.Transparent)
          .fontColor($r('app.color.primary'))
          .borderWidth(1)
          .borderColor($r('app.color.primary'))

        Button(this.showWireframe ? 'Solid' : 'Wire')
          .onClick(() => this.toggleWireframe())
          .height(32)
          .fontSize(12)
          .backgroundColor(Color.Transparent)
          .fontColor($r('app.color.primary'))
          .borderWidth(1)
          .borderColor($r('app.color.primary'))

        Button('Reset')
          .onClick(() => this.resetView())
          .height(32)
          .fontSize(12)
          .backgroundColor(Color.Transparent)
          .fontColor($r('app.color.text_secondary'))
          .borderWidth(1)
          .borderColor($r('app.color.text_secondary'))
      }
      .width('100%')
      .justifyContent(FlexAlign.SpaceEvenly)
      .padding({ left: 8, right: 8, top: 8, bottom: 12 })
      .backgroundColor($r('app.color.card_background'))
    }
    .width('100%')
    .height('100%')
  }
}
