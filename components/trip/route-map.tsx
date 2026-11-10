"use client";

import "leaflet/dist/leaflet.css";
import { useEffect, useMemo, useRef, useState, type RefObject } from "react";
import { MapContainer, TileLayer, WMSTileLayer, LayersControl, Polyline, CircleMarker, Tooltip, useMap, useMapEvents } from "react-leaflet";
import { DomEvent, type Control, type LatLngBoundsExpression } from "leaflet";
import { fetchCampsitePois, trackBbox, type CampPoi } from "@/lib/overpass";
import { MAP_COLORS, isLoopTrack, groupByLocation } from "@/lib/map-style";
import { forecastWindow, type ForecastWindow } from "@/lib/forecast-window";

type Site = { night: number; lat: number | null; lon: number | null };

// GIBS snow-cover imagery lags a day; request yesterday (UTC) for coverage.
function gibsDate(): string {
  return new Date(Date.now() - 864e5).toISOString().slice(0, 10);
}

const WAQI_TOKEN = process.env.NEXT_PUBLIC_WAQI_TOKEN;

const OVERLAY = {
  radar: "Radar now (US)",
  forecast: "Forecast precip (US)",
  snowDepth: "Snow depth (US)",
  snowGlobal: "Snow cover (global)",
  airQuality: "Air quality",
} as const;

type Legend = { name: string; title: string; note: string; stops: readonly (readonly [string, string])[] };

// Scales come from each publisher's own legend so the swatches match the tiles.
const LEGENDS: readonly Legend[] = [
  { name: OVERLAY.radar, title: "Radar now", note: "Light to heavy. Mountains can block radar.", stops: [["Light", "#536aa4"], ["", "#58c2b9"], ["", "#0daf12"], ["", "#f5cb17"], ["", "#d10809"], ["Heavy", "#f175fe"]] },
  { name: OVERLAY.forecast, title: "Forecast, next 5 days", note: "Inches of rain and melted snow", stops: [["0.01", "#7fff00"], ["0.25", "#088b00"], ["0.5", "#104e8b"], ["1", "#00b2ee"], ["1.5", "#8968cd"], ["2", "#8b008b"], ["3", "#ff0000"], ["5+", "#ff7f00"]] },
  { name: OVERLAY.snowDepth, title: "Snow depth", note: "Inches, modeled daily by NOAA", stops: [["<2", "#abc1bf"], ["2", "#66c1c4"], ["4", "#63a9cb"], ["10", "#5079c8"], ["20", "#3c3fc2"], ["39", "#5720c3"], ["98+", "#b404b1"]] },
  { name: OVERLAY.snowGlobal, title: "Snow cover", note: "Yesterday's satellite pass. Gaps are clouds.", stops: [] },
  { name: OVERLAY.airQuality, title: "Air quality", note: "US AQI at nearby monitoring stations", stops: [["0", "#009966"], ["51", "#ffde33"], ["101", "#ff9933"], ["151", "#cc0033"], ["201", "#660099"], ["301+", "#7e0023"]] },
];

// Leaflet opens this menu on hover and closes it the moment the pointer leaves,
// which makes it easy to lose. Open it on click; close on a click elsewhere.
function ClickToOpenLayers({ control }: { control: RefObject<Control.Layers | null> }) {
  useEffect(() => {
    const layers = control.current as (Control.Layers & { _expandSafely: () => void; collapse: () => void }) | null;
    const container = layers?.getContainer();
    if (!layers || !container) return;
    DomEvent.off(container, "mouseenter", layers._expandSafely, layers);
    DomEvent.off(container, "mouseleave", layers.collapse, layers);
    const closeOnOutsidePress = (event: PointerEvent) => {
      if (!container.contains(event.target as Node)) layers.collapse();
    };
    document.addEventListener("pointerdown", closeOnOutsidePress);
    return () => document.removeEventListener("pointerdown", closeOnOutsidePress);
  }, [control]);
  return null;
}

function OverlayLegend({ forecast }: { forecast: ForecastWindow }) {
  const [active, setActive] = useState<string[]>([]);
  useMapEvents({
    overlayadd: (e) => setActive((names) => [...names, e.name]),
    overlayremove: (e) => setActive((names) => names.filter((name) => name !== e.name)),
  });
  const shown = LEGENDS.filter((legend) => active.includes(legend.name)).map((legend) =>
    legend.name === OVERLAY.forecast ? { ...legend, title: forecast.title, note: forecast.note } : legend,
  );
  if (!shown.length) return null;
  return (
    <div className="pointer-events-none absolute bottom-6 left-2 z-[1000] max-w-[calc(100%-1rem)] space-y-2 rounded-xl bg-white/95 px-3 py-2 text-[#18261f] shadow">
      {shown.map((legend) => (
        <div key={legend.name}>
          <div className="text-xs font-bold">{legend.title}</div>
          <div className="text-[11px] text-[#44524a]">{legend.note}</div>
          {legend.stops.length > 0 && (
            <div className="mt-1 flex">
              {legend.stops.map(([label, color]) => (
                <div key={color} className="w-8 text-[10px] leading-tight tabular-nums">
                  <div className="h-2" style={{ background: color }} />
                  <div className="mt-0.5 whitespace-nowrap">{label}</div>
                </div>
              ))}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}

export default function RouteMap({
  track,
  campsites = [],
  preview = null,
  className,
  fill = false,
  startDate = null,
  nights = null,
}: {
  startDate?: string | null;
  nights?: number | null;
  track: [number, number][];
  className?: string;
  fill?: boolean;
  campsites?: Site[];
  preview?: { lat: number; lon: number } | null;
}) {
  const [pois, setPois] = useState<CampPoi[]>([]);
  const layersControl = useRef<Control.Layers | null>(null);
  const [forecastStart, setForecastStart] = useState(() => new Date().toISOString().slice(0, 10));
  const forecast = forecastWindow(startDate, nights, forecastStart);
  const forecastParams = useMemo(
    () => ({ layers: forecast.layer, format: "image/png", transparent: true, version: "1.3.0" }),
    [forecast.layer],
  );

  // NOAA's forecast periods roll forward with each issue, so ask where they start now.
  useEffect(() => {
    let live = true;
    fetch("https://mapservices.weather.noaa.gov/vector/rest/services/precip/wpc_qpf/MapServer/1/query?where=1%3D1&outFields=start_time&returnGeometry=false&resultRecordCount=1&f=json")
      .then((response) => response.json())
      .then((data) => {
        const start: unknown = data?.features?.[0]?.attributes?.start_time;
        if (live && typeof start === "string" && /^\d{4}-\d{2}-\d{2}/.test(start)) setForecastStart(start.slice(0, 10));
      })
      .catch(() => {});
    return () => { live = false; };
  }, []);
  const first = track[0];
  const last = track[track.length - 1];
  // Loop trip: start and end sit on top of each other — show one combined pin.
  const isLoop = isLoopTrack(track);
  const sig = track.length ? `${track.length}:${first[0]},${first[1]}:${last[0]},${last[1]}` : "";
  useEffect(() => {
    const bbox = trackBbox(track);
    if (!bbox) return;
    let live = true;
    fetchCampsitePois(bbox).then((p) => live && setPois(p));
    return () => { live = false; };
  }, [sig]); // eslint-disable-line react-hooks/exhaustive-deps

  if (!track.length) return null;
  const lats = track.map((t) => t[0]);
  const lons = track.map((t) => t[1]);
  const bounds: LatLngBoundsExpression = [
    [Math.min(...lats), Math.min(...lons)],
    [Math.max(...lats), Math.max(...lons)],
  ];
  return (
    <MapContainer bounds={bounds} scrollWheelZoom={false} className={className} style={{ height: fill ? "100%" : "20rem", width: "100%" }}>
      <ResizeMap />
      <LayersControl ref={layersControl} position="topright">
        <LayersControl.BaseLayer checked name="Topo">
          <TileLayer
            attribution="&copy; USGS The National Map"
            url="https://basemap.nationalmap.gov/arcgis/rest/services/USGSTopo/MapServer/tile/{z}/{y}/{x}"
            maxNativeZoom={16}
          />
        </LayersControl.BaseLayer>
        <LayersControl.BaseLayer name="Map">
          <TileLayer
            attribution='&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
            url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
          />
        </LayersControl.BaseLayer>

        <LayersControl.Overlay name={OVERLAY.radar}>
          <WMSTileLayer
            attribution="NOAA MRMS"
            url="https://opengeo.ncep.noaa.gov/geoserver/conus/conus_bref_qcd/ows"
            layers="conus_bref_qcd"
            format="image/png"
            transparent
            version="1.3.0"
            opacity={0.7}
          />
        </LayersControl.Overlay>
        <LayersControl.Overlay name={OVERLAY.forecast}>
          <WMSTileLayer
            attribution="NOAA Weather Prediction Center"
            url="https://mapservices.weather.noaa.gov/vector/services/precip/wpc_qpf/MapServer/WMSServer"
            params={forecastParams}
            opacity={0.55}
          />
        </LayersControl.Overlay>
        <LayersControl.Overlay name={OVERLAY.snowDepth}>
          <WMSTileLayer
            attribution="NOAA NOHRSC"
            url="https://mapservices.weather.noaa.gov/raster/services/snow/NOHRSC_Snow_Analysis/MapServer/WMSServer"
            layers="1"
            format="image/png"
            transparent
            version="1.3.0"
            opacity={0.65}
          />
        </LayersControl.Overlay>
        <LayersControl.Overlay name={OVERLAY.snowGlobal}>
          <TileLayer
            attribution="&copy; NASA GIBS"
            url={`https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_NDSI_Snow_Cover/default/${gibsDate()}/GoogleMapsCompatible_Level8/{z}/{y}/{x}.png`}
            maxNativeZoom={8}
            opacity={0.55}
          />
        </LayersControl.Overlay>
        {WAQI_TOKEN && (
          <LayersControl.Overlay name={OVERLAY.airQuality}>
            <TileLayer
              attribution='&copy; <a href="https://waqi.info">WAQI</a>'
              url={`https://tiles.waqi.info/tiles/usepa-aqi/{z}/{x}/{y}.png?token=${WAQI_TOKEN}`}
              opacity={0.5}
            />
          </LayersControl.Overlay>
        )}
      </LayersControl>
      <ClickToOpenLayers control={layersControl} />
      <OverlayLegend forecast={forecast} />
      <Polyline positions={track} pathOptions={{ color: "var(--route)", weight: 4 }} />
      {groupByLocation(campsites.filter((c) => c.lat != null && c.lon != null)).map((grp) => {
        const label = grp.map((c) => c.night).join("/");
        return (
          <CircleMarker
            key={label}
            center={[grp[0].lat!, grp[0].lon!]}
            radius={9}
            pathOptions={{ color: "#ffffff", weight: 2, fillColor: "#17382d", fillOpacity: 1 }}
          >
            <Tooltip permanent direction="top" offset={[0, -6]}>
              <span style={{ fontWeight: 700, color: "#17382d" }}>{label}</span>
            </Tooltip>
          </CircleMarker>
        );
      })}
      {/* Draw the smaller API campsite marker after the custom marker so both remain visible at the same coordinates. */}
      {pois.map((p, i) => (
        <CircleMarker
          key={`poi${i}`}
          center={[p.lat, p.lon]}
          radius={5}
          pathOptions={{ color: "#5b4a20", weight: 1.5, fillColor: "#a08a4e", fillOpacity: 0.9 }}
        >
          <Tooltip direction="top" offset={[0, -6]}>
            {p.name ?? "Campsite"}
            {p.kind === "wilderness_hut" ? " (hut)" : ""}
          </Tooltip>
        </CircleMarker>
      ))}
      {isLoop ? (
        <CircleMarker center={first} radius={8} pathOptions={{ color: MAP_COLORS.end, weight: 3, fillColor: MAP_COLORS.start, fillOpacity: 1 }}>
          <Tooltip permanent direction="top" offset={[0, -6]}>
            <span style={{ fontWeight: 700 }}>
              <span style={{ color: MAP_COLORS.start }}>Start</span> / <span style={{ color: MAP_COLORS.end }}>End</span>
            </span>
          </Tooltip>
        </CircleMarker>
      ) : (
        <>
          <CircleMarker center={first} radius={8} pathOptions={{ color: MAP_COLORS.startRing, weight: 2, fillColor: MAP_COLORS.start, fillOpacity: 1 }}>
            <Tooltip permanent direction="top" offset={[0, -6]}>
              <span style={{ fontWeight: 700, color: MAP_COLORS.start }}>Start</span>
            </Tooltip>
          </CircleMarker>
          {track.length > 1 && (
            <CircleMarker center={last} radius={8} pathOptions={{ color: MAP_COLORS.endRing, weight: 2, fillColor: MAP_COLORS.end, fillOpacity: 1 }}>
              <Tooltip permanent direction="top" offset={[0, -6]}>
                <span style={{ fontWeight: 700, color: MAP_COLORS.end }}>End</span>
              </Tooltip>
            </CircleMarker>
          )}
        </>
      )}
      {preview && (
        <CircleMarker
          center={[preview.lat, preview.lon]}
          radius={7}
          pathOptions={{ color: "var(--route)", weight: 2, fillColor: "var(--route)", fillOpacity: 0.6 }}
        />
      )}
    </MapContainer>
  );
}

function ResizeMap() {
  const map = useMap();
  useEffect(() => {
    const observer = new ResizeObserver(() => map.invalidateSize());
    observer.observe(map.getContainer());
    map.invalidateSize();
    return () => observer.disconnect();
  }, [map]);
  return null;
}
