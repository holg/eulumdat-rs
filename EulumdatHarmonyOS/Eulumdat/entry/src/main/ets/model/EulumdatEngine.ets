/**
 * Eulumdat Engine - ArkTS wrapper for native NAPI module
 *
 * This module provides a TypeScript-friendly interface to the native
 * Rust library via the NAPI bridge.
 */

import eulumdat_napi from 'libeulumdat_napi.so';

/**
 * Luminaire information from parsed file
 */
export interface LuminaireInfo {
  luminaireName: string;
  identification: string;
  luminaireNumber: string;
  fileName: string;
  dateUser: string;
  measurementReportNumber: string;
  symmetry: number;
  typeIndicator: number;
  length: number;
  width: number;
  height: number;
  maxIntensity: number;
  totalLuminousFlux: number;
  downwardFluxFraction: number;
  lightOutputRatio: number;
  numCPlanes: number;
  numGPlanes: number;
}

/**
 * Lamp set information
 */
export interface LampSetInfo {
  numLamps: number;
  lampType: string;
  totalLuminousFlux: number;
  colorAppearance: string;
  colorRenderingGroup: string;
  wattageWithBallast: number;
}

/**
 * Validation warning from file validation
 */
export interface ValidationWarning {
  code: string;
  message: string;
  severity: number;  // 0=Info, 1=Warning, 2=Error
}

/**
 * SVG theme options
 */
export enum SvgTheme {
  Light = 0,
  Dark = 1
}

/**
 * Symmetry type
 */
export enum Symmetry {
  None = 0,
  VerticalAxis = 1,
  PlaneC0C180 = 2,
  PlaneC90C270 = 3,
  BothPlanes = 4
}

/**
 * Type indicator
 */
export enum TypeIndicator {
  PointSourceSymmetric = 0,
  Linear = 1,
  PointSourceOther = 2
}

/**
 * Color mode for 3D mesh rendering
 */
export enum ColorMode {
  Heatmap = 0,      // Blue -> Cyan -> Green -> Yellow -> Red based on intensity
  CPlaneRainbow = 1, // Rainbow colors based on C-plane angle
  Solid = 2         // Solid blue color
}

/**
 * 3D mesh data for the photometric solid (LDC)
 */
export interface LdcMeshData {
  /** Vertex positions as Float32Array [x0, y0, z0, x1, y1, z1, ...] */
  positions: Float32Array;
  /** Vertex normals as Float32Array [nx0, ny0, nz0, nx1, ny1, nz1, ...] */
  normals: Float32Array;
  /** Vertex colors as Float32Array [r0, g0, b0, a0, r1, g1, b1, a1, ...] (0.0-1.0) */
  colors: Float32Array;
  /** Triangle indices as Uint32Array */
  indices: Uint32Array;
  /** Number of vertices */
  vertexCount: number;
  /** Number of indices (3 per triangle) */
  indexCount: number;
}

/**
 * EulumdatEngine - Main API for working with photometric data
 */
export class EulumdatEngine {
  private static instance: EulumdatEngine | null = null;

  /**
   * Get the singleton instance
   */
  public static getInstance(): EulumdatEngine {
    if (EulumdatEngine.instance === null) {
      EulumdatEngine.instance = new EulumdatEngine();
    }
    return EulumdatEngine.instance;
  }

  /**
   * Parse LDT (EULUMDAT) format content
   *
   * @param content The LDT file content as a string
   * @returns true if parsing succeeded
   * @throws Error if parsing fails
   */
  public parseLdt(content: string): boolean {
    return eulumdat_napi.parseLdt(content);
  }

  /**
   * Parse IES format content
   *
   * @param content The IES file content as a string
   * @returns true if parsing succeeded
   * @throws Error if parsing fails
   */
  public parseIes(content: string): boolean {
    return eulumdat_napi.parseIes(content);
  }

  /**
   * Check if a file is currently loaded
   */
  public isLoaded(): boolean {
    return eulumdat_napi.isLoaded();
  }

  /**
   * Get luminaire information from loaded file
   */
  public getInfo(): LuminaireInfo {
    return eulumdat_napi.getInfo();
  }

  /**
   * Get lamp set information from loaded file
   */
  public getLampSets(): LampSetInfo[] {
    return eulumdat_napi.getLampSets();
  }

  /**
   * Generate polar diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public polarSvg(width: number = 400, height: number = 400, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.polarSvg(width, height, theme);
  }

  /**
   * Generate cartesian diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param maxCurves Maximum number of curves to display
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public cartesianSvg(width: number = 500, height: number = 300, maxCurves: number = 8, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.cartesianSvg(width, height, maxCurves, theme);
  }

  /**
   * Generate butterfly (3D) diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param tiltDegrees Tilt angle in degrees
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public butterflySvg(width: number = 400, height: number = 400, tiltDegrees: number = 60, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.butterflySvg(width, height, tiltDegrees, theme);
  }

  /**
   * Generate heatmap diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public heatmapSvg(width: number = 400, height: number = 300, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.heatmapSvg(width, height, theme);
  }

  /**
   * Generate BUG (Backlight, Uplight, Glare) rating diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public bugSvg(width: number = 400, height: number = 350, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.bugSvg(width, height, theme);
  }

  /**
   * Generate LCS (Luminaire Classification System) diagram SVG
   *
   * @param width SVG width in pixels
   * @param height SVG height in pixels
   * @param theme Light or Dark theme
   * @returns SVG string
   */
  public lcsSvg(width: number = 500, height: number = 400, theme: SvgTheme = SvgTheme.Light): string {
    return eulumdat_napi.lcsSvg(width, height, theme);
  }

  /**
   * Export to LDT format
   */
  public exportLdt(): string {
    return eulumdat_napi.exportLdt();
  }

  /**
   * Export to IES format
   */
  public exportIes(): string {
    return eulumdat_napi.exportIes();
  }

  /**
   * Validate the loaded file (warnings)
   */
  public validate(): ValidationWarning[] {
    return eulumdat_napi.validate();
  }

  /**
   * Get validation errors (strict/fatal issues)
   */
  public getValidationErrors(): ValidationWarning[] {
    return eulumdat_napi.getValidationErrors();
  }

  /**
   * Sample intensity at given angles
   *
   * @param cAngle C-plane angle in degrees (0-360)
   * @param gAngle Gamma angle in degrees (0-180)
   * @returns Intensity in cd/klm
   */
  public sampleIntensity(cAngle: number, gAngle: number): number {
    return eulumdat_napi.sampleIntensity(cAngle, gAngle);
  }

  /**
   * Get C-plane angles array from the loaded file
   */
  public getCAngles(): number[] {
    return eulumdat_napi.getCAngles();
  }

  /**
   * Get gamma angles array from the loaded file
   */
  public getGAngles(): number[] {
    return eulumdat_napi.getGAngles();
  }

  /**
   * Get intensity at given indices
   *
   * @param cIndex C-plane index
   * @param gIndex Gamma angle index
   * @returns Intensity in cd/klm
   */
  public getIntensityAt(cIndex: number, gIndex: number): number {
    return eulumdat_napi.getIntensityAt(cIndex, gIndex);
  }

  /**
   * Get human-readable symmetry name
   */
  public symmetryName(symmetry: number): string {
    return eulumdat_napi.symmetryName(symmetry);
  }

  /**
   * Get human-readable type indicator name
   */
  public typeIndicatorName(typeIndicator: number): string {
    return eulumdat_napi.typeIndicatorName(typeIndicator);
  }

  /**
   * Generate 3D mesh data for the photometric solid (LDC)
   *
   * The mesh represents the luminous intensity distribution as a 3D surface
   * where the distance from center equals the intensity at that angle.
   *
   * @param cStep C-plane angle step in degrees (smaller = smoother, default 10)
   * @param gStep Gamma angle step in degrees (smaller = smoother, default 10)
   * @param scale Scale factor for the mesh (default 1.0)
   * @param colorMode How to color the vertices (default Heatmap)
   * @returns Mesh data with positions, normals, colors, and indices
   */
  public generateLdcMesh(
    cStep: number = 10,
    gStep: number = 10,
    scale: number = 1.0,
    colorMode: ColorMode = ColorMode.Heatmap
  ): LdcMeshData {
    return eulumdat_napi.generateLdcMesh(cStep, gStep, scale, colorMode);
  }
}
