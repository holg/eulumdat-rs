/**
 * Eulumdat - Main UI Page
 *
 * This is the main entry point for the Eulumdat HarmonyOS app.
 * It provides a tabbed interface for viewing photometric data.
 */

import { picker } from '@kit.CoreFileKit';
import { fileIo } from '@kit.CoreFileKit';
import { common } from '@kit.AbilityKit';
import { promptAction } from '@kit.ArkUI';
import { EulumdatEngine, LuminaireInfo, ValidationWarning, SvgTheme } from '../model/EulumdatEngine';

/**
 * Diagram type for tab selection
 */
enum DiagramType {
  Polar,
  Cartesian,
  Butterfly,
  Heatmap
}

@Entry
@Component
struct Index {
  @State engine: EulumdatEngine = EulumdatEngine.getInstance();
  @State isLoaded: boolean = false;
  @State luminaireInfo: LuminaireInfo | null = null;
  @State validationWarnings: ValidationWarning[] = [];
  @State currentTab: number = 0;
  @State currentDiagram: DiagramType = DiagramType.Polar;
  @State svgContent: string = '';
  @State isDarkMode: boolean = false;

  private context = getContext(this) as common.UIAbilityContext;

  aboutToAppear(): void {
    // Check initial theme
    this.isDarkMode = false; // Could detect system theme here
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
      promptAction.showToast({ message: 'Failed to select file' });
    }
  }

  /**
   * Load and parse the selected file
   */
  async loadFile(uri: string): Promise<void> {
    try {
      // Read file content
      const file = fileIo.openSync(uri, fileIo.OpenMode.READ_ONLY);
      const stat = fileIo.statSync(file.fd);
      const buffer = new ArrayBuffer(stat.size);
      fileIo.readSync(file.fd, buffer);
      fileIo.closeSync(file);

      // Convert to string (assuming UTF-8 or Latin-1)
      const content = this.arrayBufferToString(buffer);

      // Determine format from extension
      const isIes = uri.toLowerCase().endsWith('.ies');

      // Parse the file
      if (isIes) {
        this.engine.parseIes(content);
      } else {
        this.engine.parseLdt(content);
      }

      // Update state
      this.isLoaded = true;
      this.luminaireInfo = this.engine.getInfo();
      this.validationWarnings = this.engine.validate();

      // Generate initial diagram
      this.updateDiagram();

      promptAction.showToast({ message: 'File loaded successfully' });
    } catch (err) {
      console.error('Failed to load file:', err);
      promptAction.showToast({ message: `Failed to load file: ${err}` });
    }
  }

  /**
   * Convert ArrayBuffer to string
   */
  private arrayBufferToString(buffer: ArrayBuffer): string {
    const uint8Array = new Uint8Array(buffer);
    let result = '';
    for (let i = 0; i < uint8Array.length; i++) {
      result += String.fromCharCode(uint8Array[i]);
    }
    return result;
  }

  /**
   * Update the current diagram
   */
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
      case DiagramType.Heatmap:
        this.svgContent = this.engine.heatmapSvg(width + 100, height - 50, theme);
        break;
    }
  }

  /**
   * Export to LDT format
   */
  exportLdt(): void {
    if (!this.isLoaded) return;

    const ldt = this.engine.exportLdt();
    // In a real app, you would save this to a file
    console.info('LDT Export:', ldt.length, 'bytes');
    promptAction.showToast({ message: 'Exported as LDT' });
  }

  /**
   * Export to IES format
   */
  exportIes(): void {
    if (!this.isLoaded) return;

    const ies = this.engine.exportIes();
    // In a real app, you would save this to a file
    console.info('IES Export:', ies.length, 'bytes');
    promptAction.showToast({ message: 'Exported as IES' });
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
        // Empty state
        this.emptyState()
      } else {
        // Tab bar
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
    Column() {
      Image($r('app.media.foreground'))
        .width(120)
        .height(120)
        .opacity(0.5)

      Text($r('app.string.no_file_loaded'))
        .fontSize(20)
        .fontColor($r('app.color.text_secondary'))
        .margin({ top: 16 })

      Text($r('app.string.load_ldt_ies'))
        .fontSize(14)
        .fontColor($r('app.color.text_secondary'))
        .margin({ top: 8 })
    }
    .width('100%')
    .layoutWeight(1)
    .justifyContent(FlexAlign.Center)
  }

  @Builder
  infoTab() {
    if (this.luminaireInfo === null) return;

    Scroll() {
      Column() {
        this.infoCard('Luminaire Name', this.luminaireInfo.luminaireName)
        this.infoCard('Manufacturer', this.luminaireInfo.identification)
        this.infoCard('File Name', this.luminaireInfo.fileName)
        this.infoCard('Symmetry', this.engine.symmetryName(this.luminaireInfo.symmetry))
        this.infoCard('Type', this.engine.typeIndicatorName(this.luminaireInfo.typeIndicator))
        this.infoCard('Dimensions',
          `${this.luminaireInfo.length.toFixed(0)} x ${this.luminaireInfo.width.toFixed(0)} x ${this.luminaireInfo.height.toFixed(0)} mm`)
        this.infoCard('Max Intensity', `${this.luminaireInfo.maxIntensity.toFixed(1)} cd/klm`)
        this.infoCard('Total Flux', `${this.luminaireInfo.totalLuminousFlux.toFixed(0)} lm`)
        this.infoCard('Light Output Ratio', `${(this.luminaireInfo.lightOutputRatio * 100).toFixed(1)}%`)
        this.infoCard('C-Planes', `${this.luminaireInfo.numCPlanes}`)
        this.infoCard('G-Angles', `${this.luminaireInfo.numGPlanes}`)
      }
      .padding(16)
    }
    .width('100%')
    .layoutWeight(1)
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
      // Diagram type selector
      Row() {
        this.diagramButton('Polar', DiagramType.Polar)
        this.diagramButton('Cartesian', DiagramType.Cartesian)
        this.diagramButton('Butterfly', DiagramType.Butterfly)
        this.diagramButton('Heatmap', DiagramType.Heatmap)
      }
      .width('100%')
      .justifyContent(FlexAlign.SpaceEvenly)
      .padding(8)

      // SVG Display
      if (this.svgContent.length > 0) {
        // Note: In real HarmonyOS, you would use a WebView or native SVG component
        // For now, we just show a placeholder indicating the SVG is ready
        Column() {
          Text('SVG Diagram Ready')
            .fontSize(16)
            .fontColor($r('app.color.text_secondary'))

          Text(`${this.svgContent.length} bytes`)
            .fontSize(12)
            .fontColor($r('app.color.text_secondary'))
            .margin({ top: 4 })

          // In a real implementation, you would render the SVG here
          // using Web({ src: 'data:image/svg+xml;utf8,' + encodeURIComponent(this.svgContent) })
        }
        .width('100%')
        .layoutWeight(1)
        .justifyContent(FlexAlign.Center)
        .backgroundColor($r('app.color.card_background'))
        .borderRadius(12)
        .margin(16)
      }
    }
    .width('100%')
    .layoutWeight(1)
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
          ForEach(this.validationWarnings, (warning: ValidationWarning) => {
            this.warningCard(warning)
          })
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
