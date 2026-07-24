import type {
  Coordinate,
  IChartApi,
  IPrimitivePaneRenderer,
  IPrimitivePaneView,
  ISeriesApi,
  ISeriesPrimitive,
  SeriesAttachedParameter,
  SeriesType,
  Time,
} from "lightweight-charts";
import type { CanvasRenderingTarget2D } from "fancy-canvas";

export type TrendlineEndpoint = { time: Time; price: number };

export type TrendlineStyle = {
  color: string;
  /** Dashed while the pattern is still forming, solid once confirmed. */
  dashed: boolean;
  /** Short label drawn at the line's midpoint, e.g. "Neckline". */
  label?: string;
};

type ScreenPoint = { x: Coordinate | null; y: Coordinate | null };

class TrendlinePaneRenderer implements IPrimitivePaneRenderer {
  constructor(
    private readonly p1: ScreenPoint,
    private readonly p2: ScreenPoint,
    private readonly style: TrendlineStyle,
  ) {}

  draw(target: CanvasRenderingTarget2D): void {
    const { p1, p2, style } = this;
    if (p1.x === null || p1.y === null || p2.x === null || p2.y === null) {
      return;
    }
    target.useMediaCoordinateSpace(({ context }) => {
      context.save();
      context.strokeStyle = style.color;
      context.lineWidth = 1.5;
      context.setLineDash(style.dashed ? [4, 3] : []);
      context.beginPath();
      context.moveTo(p1.x as number, p1.y as number);
      context.lineTo(p2.x as number, p2.y as number);
      context.stroke();

      if (style.label) {
        context.setLineDash([]);
        context.fillStyle = style.color;
        context.font = "11px sans-serif";
        const midX = ((p1.x as number) + (p2.x as number)) / 2;
        const midY = ((p1.y as number) + (p2.y as number)) / 2;
        context.fillText(style.label, midX + 4, midY - 4);
      }
      context.restore();
    });
  }
}

class TrendlinePaneView implements IPrimitivePaneView {
  constructor(private readonly source: TrendlinePrimitive) {}

  renderer(): IPrimitivePaneRenderer | null {
    const { p1, p2 } = this.source.coordinates();
    return new TrendlinePaneRenderer(p1, p2, this.source.style);
  }
}

/**
 * Draws a single straight segment between two `(time, price)` points on a
 * series — used for pattern necklines and triangle trendlines, none of
 * which `lightweight-charts`' built-in `addPriceLine` (horizontal-only) can
 * express. One instance per line; attached via `series.attachPrimitive()`
 * and torn down automatically when the chart is removed.
 */
export class TrendlinePrimitive implements ISeriesPrimitive<Time> {
  private chart: IChartApi | null = null;
  private series: ISeriesApi<SeriesType, Time> | null = null;
  private readonly views: readonly TrendlinePaneView[];

  constructor(
    private readonly from: TrendlineEndpoint,
    private readonly to: TrendlineEndpoint,
    public readonly style: TrendlineStyle,
  ) {
    this.views = [new TrendlinePaneView(this)];
  }

  attached({ chart, series }: SeriesAttachedParameter<Time>): void {
    this.chart = chart;
    this.series = series as ISeriesApi<SeriesType, Time>;
  }

  detached(): void {
    this.chart = null;
    this.series = null;
  }

  paneViews(): readonly IPrimitivePaneView[] {
    return this.views;
  }

  coordinates(): { p1: ScreenPoint; p2: ScreenPoint } {
    if (!this.chart || !this.series) {
      return { p1: { x: null, y: null }, p2: { x: null, y: null } };
    }
    const timeScale = this.chart.timeScale();
    return {
      p1: {
        x: timeScale.timeToCoordinate(this.from.time),
        y: this.series.priceToCoordinate(this.from.price),
      },
      p2: {
        x: timeScale.timeToCoordinate(this.to.time),
        y: this.series.priceToCoordinate(this.to.price),
      },
    };
  }
}
