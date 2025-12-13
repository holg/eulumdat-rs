/**
 * Eulumdat - Main UI Page
 *
 * This is the main entry point for the Eulumdat HarmonyOS app.
 * It provides a tabbed interface for viewing photometric data.
 */

import { picker } from '@kit.CoreFileKit';
import { fileIo } from '@kit.CoreFileKit';
import { common } from '@kit.AbilityKit';
import { webview } from '@kit.ArkWeb';
import { EulumdatEngine, LuminaireInfo, LampSetInfo, ValidationWarning, SvgTheme } from '../model/EulumdatEngine';
import { Butterfly3DView } from '../components/Butterfly3DView';
import { Ldc3DView } from '../components/Ldc3DView';

/**
 * Diagram type for tab selection
 */
enum DiagramType {
  Polar,
  Cartesian,
  Butterfly,
  Butterfly3D,
  Heatmap,
  BUG,
  LCS
}

/**
 * Template definition for built-in luminaire examples
 */
interface LuminaireTemplate {
  name: string;
  description: string;
  fileName: string;
}

/**
 * Available luminaire templates
 */
const TEMPLATES: LuminaireTemplate[] = [
  { name: 'Downlight', description: 'Simple downlight with vertical axis symmetry', fileName: '1-1-0.ldt' },
  { name: 'Projector', description: 'CDM-TD 70W spotlight with asymmetric beam', fileName: 'projector.ldt' },
  { name: 'Linear', description: 'Linear luminaire with C0-C180 symmetry', fileName: '0-2-0.ldt' },
  { name: 'Fluorescent', description: 'T16 G5 54W linear luminaire', fileName: 'fluorescent_luminaire.ldt' },
  { name: 'Road Luminaire', description: 'SON-TPP 250W street light', fileName: 'road_luminaire.ldt' },
  { name: 'Floor Uplight', description: 'HIT-DE 250W floor-standing uplight', fileName: 'floor_uplight.ldt' }
];

@Entry
@Component
struct Index {
  @State engine: EulumdatEngine = EulumdatEngine.getInstance();
  @State isLoaded: boolean = false;
  @State luminaireInfo: LuminaireInfo | null = null;
  @State lampSets: LampSetInfo[] = [];
  @State validationWarnings: ValidationWarning[] = [];
  @State currentTab: number = 0;
  @State currentDiagram: DiagramType = DiagramType.Polar;
  @State svgContent: string = '';
  @State isDarkMode: boolean = false;
  @State diagramKey: number = 0;  // Key to force Web component re-render
  @State colorizeIntensities: boolean = true;  // Toggle for intensity colorization
  @State cAngles: number[] = [];  // C-plane angles from file
  @State gAngles: number[] = [];  // Gamma angles from file

  private context = getContext(this) as common.UIAbilityContext;
  private resourceManager = this.context.resourceManager;

  aboutToAppear(): void {
    this.isDarkMode = false;
  }

  /**
   * Open file picker to select LDT/IES file
   */
  async selectFile(): Promise<void> {
    try {
      const documentPicker = new picker.DocumentViewPicker(this.context);
      const result = await documentPicker.select({
        maxSelectNumber: 1,
        fileSuffixFilters: ['.ldt', '.ies', '.LDT', '.IES']
      });

      if (result.length > 0) {
        await this.loadFile(result[0]);
      }
    } catch (err) {
      console.error('Failed to select file:', err);
    }
  }

  /**
   * Load and parse the selected file
   */
  async loadFile(uri: string): Promise<void> {
    try {
      const file = fileIo.openSync(uri, fileIo.OpenMode.READ_ONLY);
      const stat = fileIo.statSync(file.fd);
      const buffer = new ArrayBuffer(stat.size);
      fileIo.readSync(file.fd, buffer);
      fileIo.closeSync(file);

      const content = this.arrayBufferToString(buffer);
      const isIes = uri.toLowerCase().endsWith('.ies');

      if (isIes) {
        this.engine.parseIes(content);
      } else {
        this.engine.parseLdt(content);
      }

      this.isLoaded = true;
      this.luminaireInfo = this.engine.getInfo();
      this.lampSets = this.engine.getLampSets();
      this.cAngles = this.engine.getCAngles();
      this.gAngles = this.engine.getGAngles();
      // Combine warnings and errors
      const warnings = this.engine.validate();
      const errors = this.engine.getValidationErrors();
      this.validationWarnings = [...errors, ...warnings];
      this.updateDiagram();
    } catch (err) {
      console.error('Failed to load file:', err);
    }
  }

  private arrayBufferToString(buffer: ArrayBuffer): string {
    const uint8Array = new Uint8Array(buffer);
    let result = '';
    for (let i = 0; i < uint8Array.length; i++) {
      result += String.fromCharCode(uint8Array[i]);
    }
    return result;
  }

  /**
   * Load a template from bundled resources
   */
  async loadTemplate(template: LuminaireTemplate): Promise<void> {
    try {
      const rawFilePath = `templates/${template.fileName}`;
      const data: Uint8Array = await this.resourceManager.getRawFileContent(rawFilePath);
      const content = this.uint8ArrayToString(data);

      this.engine.parseLdt(content);
      this.isLoaded = true;
      this.luminaireInfo = this.engine.getInfo();
      this.lampSets = this.engine.getLampSets();
      this.cAngles = this.engine.getCAngles();
      this.gAngles = this.engine.getGAngles();
      // Combine warnings and errors
      const warnings = this.engine.validate();
      const errors = this.engine.getValidationErrors();
      this.validationWarnings = [...errors, ...warnings];
      this.updateDiagram();
    } catch (err) {
      console.error('Failed to load template:', err);
    }
  }

  private uint8ArrayToString(array: Uint8Array): string {
    let result = '';
    for (let i = 0; i < array.length; i++) {
      result += String.fromCharCode(array[i]);
    }
    return result;
  }

  updateDiagram(): void {
    if (!this.isLoaded) return;

    const theme = this.isDarkMode ? SvgTheme.Dark : SvgTheme.Light;
    const width = 350;
    const height = 350;

    switch (this.currentDiagram) {
      case DiagramType.Polar:
        this.svgContent = this.engine.polarSvg(width, height, theme);
        break;
      case DiagramType.Cartesian:
        this.svgContent = this.engine.cartesianSvg(width + 100, height - 50, 8, theme);
        break;
      case DiagramType.Butterfly:
        this.svgContent = this.engine.butterflySvg(width, height, 60, theme);
        break;
      case DiagramType.Butterfly3D:
        // 3D view is handled by the Butterfly3DView component
        this.svgContent = '';
        break;
      case DiagramType.Heatmap:
        this.svgContent = this.engine.heatmapSvg(width + 100, height - 50, theme);
        break;
      case DiagramType.BUG:
        this.svgContent = this.engine.bugSvg(width, height, theme);
        break;
      case DiagramType.LCS:
        this.svgContent = this.engine.lcsSvg(width + 150, height, theme);
        break;
    }

    // Increment key to force Web component re-render
    this.diagramKey++;
  }

  exportLdt(): void {
    if (!this.isLoaded) return;
    const ldt = this.engine.exportLdt();
    console.info('LDT Export:', ldt.length, 'bytes');
  }

  exportIes(): void {
    if (!this.isLoaded) return;
    const ies = this.engine.exportIes();
    console.info('IES Export:', ies.length, 'bytes');
  }

  build() {
    Column() {
      // Header
      Row() {
        Text($r('app.string.app_name'))
          .fontSize(24)
          .fontWeight(FontWeight.Bold)
          .fontColor($r('app.color.text_primary'))

        Blank()

        Button($r('app.string.select_file'))
          .onClick(() => this.selectFile())
          .backgroundColor($r('app.color.primary'))
          .fontColor(Color.White)
      }
      .width('100%')
      .padding(16)

      if (!this.isLoaded) {
        this.emptyState()
      } else {
        Tabs({ barPosition: BarPosition.Start, index: this.currentTab }) {
          TabContent() {
            this.infoTab()
          }
          .tabBar($r('app.string.luminaire_info'))

          TabContent() {
            this.diagramsTab()
          }
          .tabBar($r('app.string.diagrams'))

          TabContent() {
            this.intensitiesTab()
          }
          .tabBar('Intensities')

          TabContent() {
            this.validationTab()
          }
          .tabBar($r('app.string.validation'))

          TabContent() {
            this.exportTab()
          }
          .tabBar($r('app.string.export'))
        }
        .onChange((index: number) => {
          this.currentTab = index;
        })
        .barMode(BarMode.Scrollable)
        .layoutWeight(1)
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor($r('app.color.background'))
  }

  @Builder
  emptyState() {
    Scroll() {
      Column() {
        Image($r('app.media.foreground'))
          .width(80)
          .height(80)
          .opacity(0.5)

        Text($r('app.string.no_file_loaded'))
          .fontSize(18)
          .fontColor($r('app.color.text_secondary'))
          .margin({ top: 12 })

        Text($r('app.string.load_ldt_ies'))
          .fontSize(13)
          .fontColor($r('app.color.text_secondary'))
          .margin({ top: 4, bottom: 16 })

        // Templates section
        Text('Or try a template:')
          .fontSize(14)
          .fontWeight(FontWeight.Medium)
          .fontColor($r('app.color.text_primary'))
          .margin({ bottom: 12 })
          .alignSelf(ItemAlign.Start)

        ForEach(TEMPLATES, (template: LuminaireTemplate) => {
          this.templateCard(template)
        })
      }
      .padding(16)
    }
    .width('100%')
    .layoutWeight(1)
  }

  @Builder
  templateCard(template: LuminaireTemplate) {
    Row() {
      Column() {
        Text(template.name)
          .fontSize(14)
          .fontWeight(FontWeight.Medium)
          .fontColor($r('app.color.text_primary'))

        Text(template.description)
          .fontSize(12)
          .fontColor($r('app.color.text_secondary'))
          .margin({ top: 4 })
          .maxLines(2)
          .textOverflow({ overflow: TextOverflow.Ellipsis })
      }
      .layoutWeight(1)
      .alignItems(HorizontalAlign.Start)

      Image($r('sys.media.ohos_ic_public_arrow_right'))
        .width(20)
        .height(20)
        .fillColor($r('app.color.text_secondary'))
    }
    .width('100%')
    .padding(12)
    .backgroundColor($r('app.color.card_background'))
    .borderRadius(8)
    .margin({ bottom: 8 })
    .onClick(() => this.loadTemplate(template))
  }

  @Builder
  infoTab() {
    if (this.luminaireInfo !== null) {
      Scroll() {
        Column() {
          // Basic Info Section
          this.sectionHeader('Luminaire')
          this.infoCard('Name', this.luminaireInfo!.luminaireName)
          this.infoCard('Manufacturer', this.luminaireInfo!.identification)
          this.infoCard('File Name', this.luminaireInfo!.fileName)

          // Dimensions Section
          this.sectionHeader('Dimensions')
          this.infoCard('Size (L×W×H)',
            `${this.luminaireInfo!.length.toFixed(0)} × ${this.luminaireInfo!.width.toFixed(0)} × ${this.luminaireInfo!.height.toFixed(0)} mm`)

          // Photometry Section
          this.sectionHeader('Photometry')
          this.infoCard('Max Intensity', `${this.luminaireInfo!.maxIntensity.toFixed(1)} cd/klm`)
          this.infoCard('Total Flux', `${this.luminaireInfo!.totalLuminousFlux.toFixed(0)} lm`)
          this.infoCard('Light Output Ratio', `${(this.luminaireInfo!.lightOutputRatio * 100).toFixed(1)}%`)
          this.infoCard('Symmetry', this.engine.symmetryName(this.luminaireInfo!.symmetry))
          this.infoCard('Type', this.engine.typeIndicatorName(this.luminaireInfo!.typeIndicator))

          // Angles Section
          this.sectionHeader('Measurement Angles')
          this.infoCard('C-Planes', `${this.luminaireInfo!.numCPlanes}`)
          this.infoCard('γ-Angles', `${this.luminaireInfo!.numGPlanes}`)

          // Lamp Sets Section
          if (this.lampSets.length > 0) {
            this.sectionHeader(`Lamp Sets (${this.lampSets.length})`)
            ForEach(this.lampSets, (lamp: LampSetInfo, index: number) => {
              this.lampSetCard(lamp, index)
            }, (lamp: LampSetInfo, index: number) => `lamp_${index}`)
          }
        }
        .padding(16)
      }
      .width('100%')
      .layoutWeight(1)
    } else {
      Column() {
        Text('No data available')
          .fontSize(16)
          .fontColor($r('app.color.text_secondary'))
      }
      .width('100%')
      .layoutWeight(1)
      .justifyContent(FlexAlign.Center)
    }
  }

  @Builder
  sectionHeader(title: string) {
    Text(title)
      .fontSize(16)
      .fontWeight(FontWeight.Bold)
      .fontColor($r('app.color.primary'))
      .margin({ top: 16, bottom: 8 })
      .alignSelf(ItemAlign.Start)
  }

  @Builder
  lampSetCard(lamp: LampSetInfo, index: number) {
    Column() {
      Text(`Lamp Set ${index + 1}`)
        .fontSize(14)
        .fontWeight(FontWeight.Bold)
        .fontColor($r('app.color.text_primary'))
        .margin({ bottom: 8 })

      this.infoRow('Quantity', `${lamp.numLamps}`)
      this.infoRow('Type', lamp.lampType)
      this.infoRow('Flux', `${lamp.totalLuminousFlux.toFixed(0)} lm`)
      this.infoRow('Color Temp', lamp.colorAppearance)
      this.infoRow('CRI', lamp.colorRenderingGroup)
      this.infoRow('Power', `${lamp.wattageWithBallast.toFixed(1)} W`)
    }
    .width('100%')
    .padding(12)
    .backgroundColor($r('app.color.card_background'))
    .borderRadius(8)
    .margin({ bottom: 8 })
    .alignItems(HorizontalAlign.Start)
  }

  @Builder
  infoRow(label: string, value: string) {
    Row() {
      Text(label)
        .fontSize(12)
        .fontColor($r('app.color.text_secondary'))
        .layoutWeight(1)

      Text(value)
        .fontSize(12)
        .fontColor($r('app.color.text_primary'))
    }
    .width('100%')
    .margin({ bottom: 4 })
  }

  @Builder
  infoCard(label: string, value: string) {
    Row() {
      Text(label)
        .fontSize(14)
        .fontColor($r('app.color.text_secondary'))
        .layoutWeight(1)

      Text(value)
        .fontSize(14)
        .fontColor($r('app.color.text_primary'))
        .fontWeight(FontWeight.Medium)
    }
    .width('100%')
    .padding(12)
    .backgroundColor($r('app.color.card_background'))
    .borderRadius(8)
    .margin({ bottom: 8 })
  }

  @Builder
  diagramsTab() {
    Column() {
      // First row of diagram buttons
      Row() {
        this.diagramButton('Polar', DiagramType.Polar)
        this.diagramButton('Cartesian', DiagramType.Cartesian)
        this.diagramButton('Butterfly', DiagramType.Butterfly)
        this.diagramButton('3D', DiagramType.Butterfly3D)
      }
      .width('100%')
      .justifyContent(FlexAlign.SpaceEvenly)
      .padding({ left: 8, right: 8, top: 8 })

      // Second row of diagram buttons
      Row() {
        this.diagramButton('Heatmap', DiagramType.Heatmap)
        this.diagramButton('BUG', DiagramType.BUG)
        this.diagramButton('LCS', DiagramType.LCS)
      }
      .width('100%')
      .justifyContent(FlexAlign.SpaceEvenly)
      .padding({ left: 8, right: 8, bottom: 8 })

      // Show diagram based on current selection
      if (this.currentDiagram === DiagramType.Butterfly3D && this.luminaireInfo !== null) {
        // Use the new Ldc3DView with mesh-based rendering
        Ldc3DView({
          engine: this.engine,
          info: this.luminaireInfo!,
          isDarkMode: this.isDarkMode
        })
          .width('100%')
          .layoutWeight(1)
      } else if (this.currentDiagram === DiagramType.Polar && this.svgContent.length > 0) {
        this.svgWebView()
      } else if (this.currentDiagram === DiagramType.Cartesian && this.svgContent.length > 0) {
        this.svgWebView()
      } else if (this.currentDiagram === DiagramType.Butterfly && this.svgContent.length > 0) {
        this.svgWebView()
      } else if (this.currentDiagram === DiagramType.Heatmap && this.svgContent.length > 0) {
        this.svgWebView()
      } else if (this.currentDiagram === DiagramType.BUG && this.svgContent.length > 0) {
        this.svgWebView()
      } else if (this.currentDiagram === DiagramType.LCS && this.svgContent.length > 0) {
        this.svgWebView()
      } else {
        Column() {
          Text('Select a diagram type')
            .fontSize(16)
            .fontColor($r('app.color.text_secondary'))
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
      }
    }
    .width('100%')
    .layoutWeight(1)
  }

  @Builder
  svgWebView() {
    Web({
      src: 'data:text/html,' + encodeURIComponent(this.createSvgHtml()),
      controller: new webview.WebviewController()
    })
      .width('100%')
      .layoutWeight(1)
      .backgroundColor($r('app.color.card_background'))
      .margin(16)
  }

  @Builder
  diagramButton(label: string, type: DiagramType) {
    Button(label)
      .backgroundColor(this.currentDiagram === type ? $r('app.color.primary') : Color.Transparent)
      .fontColor(this.currentDiagram === type ? Color.White : $r('app.color.primary'))
      .borderWidth(1)
      .borderColor($r('app.color.primary'))
      .onClick(() => {
        this.currentDiagram = type;
        this.updateDiagram();
      })
  }

  @Builder
  intensitiesTab() {
    Column() {
      // Header with toggle
      Row() {
        Text('Intensities (cd/klm)')
          .fontSize(16)
          .fontWeight(FontWeight.Bold)
          .fontColor($r('app.color.text_primary'))

        Blank()

        Row() {
          Text('Colorize')
            .fontSize(12)
            .fontColor($r('app.color.text_secondary'))
            .margin({ right: 8 })

          Toggle({ type: ToggleType.Switch, isOn: this.colorizeIntensities })
            .onChange((isOn: boolean) => {
              this.colorizeIntensities = isOn;
            })
        }
      }
      .width('100%')
      .padding(12)
      .backgroundColor($r('app.color.card_background'))

      // Intensity grid
      if (this.luminaireInfo !== null) {
        Scroll() {
          Column() {
            this.intensityGrid()
          }
          .padding(8)
        }
        .scrollable(ScrollDirection.Horizontal)
        .width('100%')
        .layoutWeight(1)
      }

      // Footer with legend
      Row() {
        Text(`${this.luminaireInfo?.numCPlanes ?? 0} C × ${this.luminaireInfo?.numGPlanes ?? 0} γ`)
          .fontSize(11)
          .fontColor($r('app.color.text_secondary'))

        Blank()

        if (this.colorizeIntensities) {
          Row() {
            Text('0')
              .fontSize(10)
              .fontColor($r('app.color.text_secondary'))
            ForEach([0, 1, 2, 3, 4, 5, 6, 7, 8, 9], (i: number) => {
              Row()
                .width(10)
                .height(10)
                .backgroundColor(this.heatmapColor(i / 9))
            }, (i: number) => `legend_${i}`)
            Text('max')
              .fontSize(10)
              .fontColor($r('app.color.text_secondary'))
          }
        }
      }
      .width('100%')
      .padding(8)
      .backgroundColor($r('app.color.card_background'))
    }
    .width('100%')
    .layoutWeight(1)
  }

  private getMaxIntensity(): number {
    return this.luminaireInfo?.maxIntensity ?? 1;
  }

  private getIntensityCellColorByIndex(cIdx: number, gIdx: number): string {
    if (!this.colorizeIntensities) return '#ffffff';
    const intensity = this.engine.getIntensityAt(cIdx, gIdx);
    const maxIntensity = this.getMaxIntensity();
    const normalized = maxIntensity > 0 ? intensity / maxIntensity : 0;
    return this.heatmapColor(normalized);
  }

  private getIntensityCellTextColorByIndex(cIdx: number, gIdx: number): string {
    if (!this.colorizeIntensities) return '#333333';
    const intensity = this.engine.getIntensityAt(cIdx, gIdx);
    const maxIntensity = this.getMaxIntensity();
    const normalized = maxIntensity > 0 ? intensity / maxIntensity : 0;
    return normalized > 0.5 ? '#ffffff' : '#333333';
  }

  private getIntensityValueByIndex(cIdx: number, gIdx: number): string {
    return this.engine.getIntensityAt(cIdx, gIdx).toFixed(0);
  }

  @Builder
  intensityGrid() {
    Column() {
      // Header row with C angles
      Row() {
        // Corner cell
        Text('γ\\C')
          .fontSize(10)
          .fontWeight(FontWeight.Bold)
          .width(40)
          .height(28)
          .textAlign(TextAlign.Center)
          .backgroundColor($r('app.color.card_background'))
          .border({ width: 0.5, color: $r('app.color.text_secondary') })

        ForEach(this.cAngles, (c: number, cIdx: number) => {
          Text(`C${c.toFixed(0)}°`)
            .fontSize(9)
            .fontWeight(FontWeight.Bold)
            .width(50)
            .height(28)
            .textAlign(TextAlign.Center)
            .backgroundColor($r('app.color.card_background'))
            .border({ width: 0.5, color: $r('app.color.text_secondary') })
        }, (c: number, cIdx: number) => `c_header_${cIdx}`)
      }

      // Data rows
      ForEach(this.gAngles, (g: number, gIdx: number) => {
        Row() {
          // Row header with G angle
          Text(`${g.toFixed(0)}°`)
            .fontSize(10)
            .fontWeight(FontWeight.Bold)
            .width(40)
            .height(24)
            .textAlign(TextAlign.Center)
            .backgroundColor($r('app.color.card_background'))
            .border({ width: 0.5, color: $r('app.color.text_secondary') })

          // Intensity cells
          ForEach(this.cAngles, (c: number, cIdx: number) => {
            Text(this.getIntensityValueByIndex(cIdx, gIdx))
              .fontSize(9)
              .fontColor(this.getIntensityCellTextColorByIndex(cIdx, gIdx))
              .width(50)
              .height(24)
              .textAlign(TextAlign.Center)
              .backgroundColor(this.getIntensityCellColorByIndex(cIdx, gIdx))
              .border({ width: 0.5, color: '#cccccc' })
          }, (c: number, cIdx: number) => `cell_${gIdx}_${cIdx}`)
        }
      }, (g: number, gIdx: number) => `g_row_${gIdx}`)
    }
  }

  /**
   * Generate heatmap color from normalized value (0-1)
   */
  private heatmapColor(normalized: number): string {
    const value = Math.max(0, Math.min(1, normalized));
    let r: number, g: number, b: number;

    if (value < 0.25) {
      const t = value / 0.25;
      r = 0; g = Math.round(t * 255); b = 255;
    } else if (value < 0.5) {
      const t = (value - 0.25) / 0.25;
      r = 0; g = 255; b = Math.round((1 - t) * 255);
    } else if (value < 0.75) {
      const t = (value - 0.5) / 0.25;
      r = Math.round(t * 255); g = 255; b = 0;
    } else {
      const t = (value - 0.75) / 0.25;
      r = 255; g = Math.round((1 - t) * 255); b = 0;
    }

    const toHex = (v: number): string => {
      const hex = v.toString(16);
      return hex.length === 1 ? '0' + hex : hex;
    };

    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  }

  @Builder
  validationTab() {
    if (this.validationWarnings.length === 0) {
      Column() {
        Text($r('app.string.no_issues'))
          .fontSize(16)
          .fontColor($r('app.color.text_secondary'))
      }
      .width('100%')
      .layoutWeight(1)
      .justifyContent(FlexAlign.Center)
    } else {
      Scroll() {
        Column() {
          ForEach(this.validationWarnings, (warning: ValidationWarning, index: number) => {
            this.warningCard(warning)
          }, (warning: ValidationWarning, index: number) => `warning_${index}_${warning.code}`)
        }
        .padding(16)
      }
      .width('100%')
      .layoutWeight(1)
    }
  }

  @Builder
  warningCard(warning: ValidationWarning) {
    Row() {
      Circle()
        .width(8)
        .height(8)
        .fill(this.severityColor(warning.severity))
        .margin({ right: 12 })

      Column() {
        Text(warning.code)
          .fontSize(14)
          .fontWeight(FontWeight.Medium)
          .fontColor($r('app.color.text_primary'))

        Text(warning.message)
          .fontSize(12)
          .fontColor($r('app.color.text_secondary'))
          .margin({ top: 4 })
      }
      .layoutWeight(1)
      .alignItems(HorizontalAlign.Start)
    }
    .width('100%')
    .padding(12)
    .backgroundColor($r('app.color.card_background'))
    .borderRadius(8)
    .margin({ bottom: 8 })
  }

  private severityColor(severity: number): ResourceColor {
    switch (severity) {
      case 0: return $r('app.color.info');
      case 1: return $r('app.color.warning');
      case 2: return $r('app.color.error');
      default: return $r('app.color.info');
    }
  }

  /**
   * Create HTML wrapper for SVG rendering in WebView
   */
  private createSvgHtml(): string {
    const bgColor = this.isDarkMode ? '#1C1C1E' : '#FFFFFF';
    return `<!DOCTYPE html>
<html>
<head>
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <style>
    * { margin: 0; padding: 0; box-sizing: border-box; }
    html, body {
      width: 100%;
      height: 100%;
      background: ${bgColor};
      display: flex;
      justify-content: center;
      align-items: center;
      overflow: hidden;
    }
    svg {
      max-width: 100%;
      max-height: 100%;
      width: auto;
      height: auto;
    }
  </style>
</head>
<body>
  ${this.svgContent}
</body>
</html>`;
  }

  @Builder
  exportTab() {
    Column() {
      Button($r('app.string.export_ldt'))
        .width('80%')
        .height(48)
        .backgroundColor($r('app.color.primary'))
        .fontColor(Color.White)
        .margin({ bottom: 16 })
        .onClick(() => this.exportLdt())

      Button($r('app.string.export_ies'))
        .width('80%')
        .height(48)
        .backgroundColor($r('app.color.primary'))
        .fontColor(Color.White)
        .onClick(() => this.exportIes())
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }
}
