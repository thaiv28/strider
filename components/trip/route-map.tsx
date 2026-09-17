"use client";

import "leaflet/dist/leaflet.css";
import { useEffect, useState } from "react";
import { MapContainer, TileLayer, LayersControl, Polyline, CircleMarker, Tooltip } from "react-leaflet";
import type { LatLngBoundsExpression } from "leaflet";
import { fetchCampsitePois, trackBbox, type CampPoi } from "@/lib/overpass";
import { MAP_COLORS, isLoopTrack, groupByLocation } from "@/lib/map-style";

type Site = { night: number; lat: number | null; lon: number | null };

// GIBS snow-cover imagery lags a day; request yesterday (UTC) for coverage.
function gibsDate(): string {
  return new Date(Date.now() - 864e5).toISOString().slice(0, 10);
}

const WAQI_TOKEN = process.env.NEXT_PUBLIC_WAQI_TOKEN;

export default function RouteMap({
  track,
  campsites = [],
  preview = null,
}: {
  track: [number, number][];
  campsites?: Site[];
  preview?: { lat: number; lon: number } | null;
}) {
  const [pois, setPois] = useState<CampPoi[]>([]);
  const [radarUrl, setRadarUrl] = useState<string | null>(null);
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

  // Latest RainViewer radar frame; the tile path rotates, so resolve it live.
  useEffect(() => {
    let live = true;
    fetch("https://api.rainviewer.com/public/weather-maps.json")
      .then((r) => r.json())
      .then((j) => {
        const frames = j?.radar?.past ?? [];
        const latest = frames[frames.length - 1];
        if (live && j?.host && latest?.path) setRadarUrl(`${j.host}${latest.path}/256/{z}/{x}/{y}/4/1_1.png`);
      })
      .catch(() => {});
    return () => { live = false; };
  }, []);

  if (!track.length) return null;
  const lats = track.map((t) => t[0]);
  const lons = track.map((t) => t[1]);
  const bounds: LatLngBoundsExpression = [
    [Math.min(...lats), Math.min(...lons)],
    [Math.max(...lats), Math.max(...lons)],
  ];
  return (
    <MapContainer bounds={bounds} scrollWheelZoom={false} style={{ height: "20rem", width: "100%" }}>
      <LayersControl position="topright">
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

        {radarUrl && (
          <LayersControl.Overlay name="Radar (precip)">
            <TileLayer attribution="&copy; RainViewer" url={radarUrl} opacity={0.6} />
          </LayersControl.Overlay>
        )}
        <LayersControl.Overlay name="Snow cover">
          <TileLayer
            attribution="&copy; NASA GIBS"
            url={`https://gibs.earthdata.nasa.gov/wmts/epsg3857/best/MODIS_Terra_Snow_Cover/default/${gibsDate()}/GoogleMapsCompatible_Level8/{z}/{y}/{x}.png`}
            maxNativeZoom={8}
            opacity={0.55}
          />
        </LayersControl.Overlay>
        {WAQI_TOKEN && (
          <LayersControl.Overlay name="Air quality">
            <TileLayer
              attribution='&copy; <a href="https://waqi.info">WAQI</a>'
              url={`https://tiles.waqi.info/tiles/usepa-aqi/{z}/{x}/{y}.png?token=${WAQI_TOKEN}`}
              opacity={0.5}
            />
          </LayersControl.Overlay>
        )}
      </LayersControl>
      <Polyline positions={track} pathOptions={{ color: MAP_COLORS.route, weight: 4 }} />
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
      {groupByLocation(campsites.filter((c) => c.lat != null && c.lon != null)).map((grp) => {
        const label = grp.map((c) => c.night).join("/");
        return (
          <CircleMarker
            key={label}
            center={[grp[0].lat!, grp[0].lon!]}
            radius={9}
            pathOptions={{ color: MAP_COLORS.campRing, weight: 2, fillColor: MAP_COLORS.camp, fillOpacity: 1 }}
          >
            <Tooltip permanent direction="top" offset={[0, -6]}>
              <span style={{ fontWeight: 700, color: MAP_COLORS.camp }}>{label}</span>
            </Tooltip>
          </CircleMarker>
        );
      })}
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
          pathOptions={{ color: MAP_COLORS.route, weight: 2, fillColor: MAP_COLORS.route, fillOpacity: 0.6 }}
        />
      )}
    </MapContainer>
  );
}
