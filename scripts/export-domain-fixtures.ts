/** Deterministic executable specifications, imported from the web's real rules. */
import { mkdirSync, writeFileSync } from "node:fs";
import * as util from "@/lib/util";
import * as food from "@/lib/food";
import { fuelPlan } from "@/lib/fuel";
import { mifflinBmr } from "@/lib/energy";
import { dayTargetKcal, effectiveRouteStats, computeDaySegments } from "@/lib/trip";
type Case = { fn: string; args: unknown[]; out: unknown };
let seed = 0x5eed1234;
const next = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed / 4294967296; };
const files: Record<string, Case[]> = {};
const add = (module: string, fn: string, f: (...args: any[]) => unknown, args: unknown[]) => (files[module] ??= []).push({ fn, args, out: f(...args) });
const weights = [null, 0, ...Array.from({length:40}, (_,i)=>i+1), ...Array.from({length:300}, ()=>41 + Math.floor(next()*19960))];
for (const name of ["fmtWeight", "fmtOz", "fmtLbs", "displayOz", "gToOz"] as const) for (const g of weights) add("util", name, util[name] as any, [g]);
for (let i = 0; i <= 1280; i++) add("util", "ozToG", util.ozToG, [i / 20]);
add("util", "ozToG", util.ozToG, [null]);
for (const value of [null,0,1,50,99,100,1250,129900,100000000,...Array.from({length:50},()=>Math.floor(next()*10000000))]) add("util","fmtUsd",util.fmtUsd,[value]);
for (const start of [null,"2026-01-15","2026-01-30","2026-12-30","2024-02-28","2024-02-29"]) for(const nights of [null,0,1,3,4,10]) add("util","fmtDateRange",util.fmtDateRange,[start,nights]);
for(const kcal of [null,0,1,100,357,450,900]) { add("food","kcalPerOz",food.kcalPerOz,[kcal]); for(const grams of [0,1,28,37.5,100,250]) add("food","kcalForGrams",food.kcalForGrams,[kcal,grams]); }
for(const unit of Object.keys(food.VOLUME_ML)) for(const density of [null,0,0.45,1,1.03]) for(const amount of [0,0.25,1,1.5,100]) { add("food","gramsFromVolume",food.gramsFromVolume as any,[amount,unit,density]); add("food","volumeFromGrams",food.volumeFromGrams as any,[amount,unit,density]); }
for(const label of ["fl oz","Fluid ounces","1 cup (240ml)","Tbsp.","xyz","cups","tablespoon","tablespoons","tbsp","tbs","teaspoon","teaspoons","tsp.","milliliter","milliliters","millilitre","millilitres","ml","liter","liters","litre","litres","l","2 FL. OZ.","fluid ounce","oz","g","cupcake","1/2 cup","fl-oz","2 teaspoon (10 ml)",""]) add("food","parseVolumeUnit",food.parseVolumeUnit,[label]);
for(const serving of [null,"","1 cup (240 g)","1 cup (120 grams)","1/2 cup (60g)","1 1/2 cup (180 g)","2 tbsp (32 g)","1 Tbsp. (16g)","1 tsp (4 g)","1 fl oz (30 g)","2 fluid ounces (59 g)","100 ml (103 g)","1 l (1000g)","1 cup","1 cup (0g)","1 cup (1g)","1 cup (1000g)","1/3 cup (40g)","0.5 cup (60 gr)","1 slice (28 g)","28 g","2 cups (480 grams)","1 teaspoon (0.1 g)","1 tablespoon (36.9 g)","1 cup (591.5g)","1 cup (4.7g)","1 cup (4.8g)","1/0 cup (20g)","1 cup (10g), 2 tbsp (30g)"]) for(const hint of [null,28,120]) add("food","densityFromServing",food.densityFromServing,[serving,hint]);
for (const value of [null,""," ","1","1.25","-2","xyz","0"]) add("food","num",food.num,[value]);
for (const value of ["g","oz","ml","l","tsp","tbsp","floz","cup","cups","xyz",""]) add("food","isAmountUnit",food.isAmountUnit,[value]);
for (const water of [0,1,500,9166,9167,19167,37500,100000,...Array.from({length:30},()=>Math.floor(next()*150000))]) add("fuel","fuelPlan",fuelPlan,[water]);
for (const sex of [null,"male","female"]) for(const body of [[null,70,30],[180,null,30],[180,70,null],[0,70,30],[180,70,30],[130,64,45],[220,76,22],[155.5,67.5,62]]) add("energy","mifflinBmr",mifflinBmr,[sex,...body]);
for(const params of [{bmr:1735,calPerMile:200,ftPerMile:625},{bmr:1400,calPerMile:180,ftPerMile:700}]) for(const distance of [null,0,5.25,15]) for(const gain of [null,0,625,2500]) add("trip","dayTargetKcal",dayTargetKcal,[params,distance,gain]);
for(let mask=0;mask<64;mask++) { const trip={distanceMi:mask&1?"0":null,elevationGainFt:mask&2?0:null}; const gpx={distanceMi:mask&4?"12.5":null,elevationGainFt:mask&8?1500:null};const trail={typicalDistanceMi:mask&16?"20":null,typicalElevationFt:mask&32?3000:null}; add("trip","effectiveRouteStats",effectiveRouteStats,[trip,gpx,trail]); }
for(const [gpx,trail] of [[null,null],[null,{typicalDistanceMi:"3",typicalElevationFt:5}],[{distanceMi:"2",elevationGainFt:4},null]]) add("trip","effectiveRouteStats",effectiveRouteStats,[{distanceMi:null,elevationGainFt:null},gpx,trail]);
const routes = [Array.from({length:11},(_,i)=>({d:i,lat:40+i/100,lon:-120-i/100,e:100})),Array.from({length:21},(_,i)=>({d:i/2,lat:40+i/100,lon:-120,e:100+[0,5,12,13,20,30,17,4,10,25,45][i%11]+i*5})),Array.from({length:501},(_,i)=>({d:i/50,lat:40+i/10000,lon:-120,e:100+i*2+((i%7)-3)*4}))];
for(const route of routes) for(let count=0;count<=4;count++) { const camps=Array.from({length:count},(_,i)=>({distanceMi:10*(i+1)/(count+1),lat:i%2?null:40+i/10,lon:i%2?null:-120}));add("trip","computeDaySegments",computeDaySegments,[route,camps]); }
add("trip","computeDaySegments",computeDaySegments,[[],[{distanceMi:1,lat:null,lon:null}]]);
// Adversarial Math.round cases: negative ties and floats immediately below halves.
for (const value of [-1.5,-0.5,-0.5000000000000001,-1e-300,1e-300,0.49999999999999994,0.5,0.5000000000000001]) {
  add("food","kcalForGrams",food.kcalForGrams,[100,value]);
  add("food","gramsFromVolume",food.gramsFromVolume,[value,"ml",1]);
  add("trip","dayTargetKcal",dayTargetKcal,[{bmr:value,calPerMile:1,ftPerMile:625},null,null]);
  add("util","ozToG",util.ozToG,[value/util.OZ_TO_G]);
}
for (const grams of [-100,-50,-35,-28,-1]) for (const name of ["fmtWeight","fmtOz","fmtLbs","displayOz","gToOz"] as const) add("util",name,util[name] as any,[grams]);
mkdirSync("desktop/fixtures/domain",{recursive:true});
for(const [name,cases] of Object.entries(files)) writeFileSync(`desktop/fixtures/domain/${name}.json`,JSON.stringify(cases,null,2)+"\n");
console.log(Object.entries(files).map(([name,cases])=>`${name}: ${cases.length}`).join("\n"));
