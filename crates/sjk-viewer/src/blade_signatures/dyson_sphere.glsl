// Dyson Sphere: the blade is a star caged in a megastructure: a white-gold plasma core seen
// through a lattice of dark hexagonal panels along the whole blade. The panels slide open and shut
// in waves, light pouring through the gaps; on a swing every panel opens and the star blazes.
// @room 8
// @solid 1
// @palette {"corona.rim_cool":[0.70,0.35,0.05],"corona.rim_hot":[1.0,0.75,0.30],"corona.inner":[1.0,0.95,0.80],"core.white":[1.0,0.97,0.88],"core.fringe_cool":[1.0,0.65,0.20],"core.fringe_hot":[1.0,0.90,0.60],"light.color":[1.0,0.75,0.35],"trail":[1.0,0.78,0.4]}

float ds_d(vec2 b){ return length(vec2(b.x, max(b.y - uLength, 0.0) + max(-b.y, 0.0))); }
// hex cell coordinates of p: (cell centre offset, id)
vec4 ds_hex(vec2 p){
  vec2 s = vec2(1.0, 1.732);
  vec2 a = mod(p, s) - s*0.5, c = mod(p - s*0.5, s) - s*0.5;
  vec2 g = dot(a, a) < dot(c, c) ? a : c;
  return vec4(g, p - g);
}
// how open the panel `id` is (0 shut, 1 open)
float ds_open(vec2 id){
  float wave = sin(id.y*0.35 - uTime*1.3 + h21(id)*2.0);
  return clamp(smoothstep(0.2, 0.9, wave) + uSwing*1.5 + uClash, 0.0, 1.0);
}
vec3 dyson_sphere_glow0(vec3 g, vec2 b, float r, float px){
  float d = ds_d(b);
  vec3 star = vec3(1.0, 0.82, 0.45);
  float core = fillD(d - 0.35, px);
  float flare = fbm3(vec2(b.x*3.0, b.y*0.6 - uTime*1.5));
  vec3 light = mix(star, vec3(1.0, 0.98, 0.9), core)*(fillD(d - 0.8, px)*(0.7 + 0.5*flare) + core*0.6);
  vec4 h = ds_hex(vec2(b.x*1.6, b.y*1.6));
  float open = ds_open(h.zw);
  float gap = smoothstep(0.42, 0.36, length(h.xy))*open;
  light += star*halo(d - 0.8, 1.2)*(0.25 + 0.6*open)*step(0.0, b.y);
  light += star*gap*fillD(d - 0.95, px)*1.2;
  return light;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
// the panels: dark metal hexes with lit seams, sliding aside as they open
vec4 sig_solid(vec4 o, vec2 b, float px){
  float d = ds_d(b);
  if (d > 1.0 || b.y < 0.0) return o;
  vec4 h = ds_hex(vec2(b.x*1.6, b.y*1.6));
  float open = ds_open(h.zw);
  float rad = length(h.xy);
  float panel = fillD(rad - mix(0.47, 0.12, open), px*1.6)*fillD(d - 0.95, px);
  float seam = smoothstep(0.06, 0.0, abs(rad - mix(0.47, 0.12, open)))*panel;
  vec3 metal = mix(vec3(0.07, 0.06, 0.05), vec3(0.32, 0.27, 0.2), 0.5 + 0.5*h.y*3.0);
  vec3 c = mix(metal, vec3(1.0, 0.8, 0.4), seam*0.8);
  return vec4(mix(o.rgb, c, panel), max(o.a, panel*0.95));
}

// ==== show (added) ====
// Dyson Sphere show: collector satellites orbit round the blade on tilted paths, each catching a
// thin beam of starlight from the blade; solar flares burst out through the open panel gaps as
// plasma jets that curl and fade, and a slow ring of hexagon panels turns round it. A swing makes
// every flare jet out; a clash blasts a hexagonal shockwave.
float dsHex(vec2 p){ p = abs(p); return max(p.x*0.866 + p.y*0.5, p.y); }
vec3 dyson_sphere_show(vec2 b, float px){
  float L = uLength, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 gold = vec3(1.0, 0.78, 0.35), white = vec3(1.0, 0.95, 0.85);
  vec3 light = vec3(0.0);
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float a = uTime*(0.5 + 0.2*fi) + fi*1.7;
    float R = 3.0 + 1.0*h11(fi*3.0);
    float yc = (0.15 + 0.13*fi)*L;
    vec2 p = vec2(R*sin(a), yc + R*0.25*cos(a));
    float front = 0.4 + 0.6*step(0.0, cos(a));
    float hx = abs(dsHex((b - p)/0.35) - 1.0)*0.35;
    light += gold*(halo(hx, 0.03 + px)*1.4 + fillD(dsHex((b - p)/0.35)*0.35 - 0.35, px)*0.25)*front;
    vec2 bp = vec2(0.0, p.y);
    light += white*halo(sdSeg(b, bp, p), 0.02 + px)*0.5*front*(0.6 + 0.4*sin(uTime*7.0 + fi));
  }
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float ph = uTime*(0.5 + 0.3*h11(fi*2.7)) + h11(fi*6.1);
    float cy = floor(ph), f = fract(ph);
    if (h11(cy*3.7 + fi) > 0.55 + 0.45*sw) continue;
    float s = h11(cy*1.9 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.8*h11(cy*2.3 + fi))*L;
    float len = (2.5 + 3.5*h11(cy + fi))*smoothstep(0.0, 0.35, f);
    float u = (b.x*s - 0.5)/max(len, 0.01);
    if (u < 0.0 || u > 1.0) continue;
    float curl = u*u*2.0*(h11(cy*5.0 + fi) - 0.5) + 0.3*sin(u*8.0 - uTime*12.0)*u;
    float dy = b.y - y0 - curl;
    float th = 0.12 + 0.3*u;
    light += mix(white, vec3(1.0, 0.45, 0.1), u)*exp(-pow(dy/th, 2.0))*(1.0 - u)*(1.0 - f)*1.6;
  }
  for (int k = 0; k < 2; k++){
    float yc = L*(0.3 + 0.4*float(k));
    vec2 q = vec2(b.x, (b.y - yc)/0.28);
    float r = length(q);
    float ang = atan(q.y, q.x) + uTime*0.4*(k == 0 ? 1.0 : -1.0);
    float cells = step(0.15, abs(fract(ang*12.0/6.2832) - 0.5))*smoothstep(0.35, 0.0, abs(r - 4.5));
    float behind = step(0.0, q.y);
    light += gold*cells*0.35*mix(1.0, 1.0 - exp(-pow(b.x/0.5, 2.0)), behind);
  }
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    float h = abs(dsHex((b - o)/(1.0 + 7.0*(1.0 - cl))) - 1.0)*(1.0 + 7.0*(1.0 - cl));
    light += white*halo(h, 0.15)*cl*1.6;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return dyson_sphere_glow0(g, b, r, px) + dyson_sphere_show(b, min(px, 0.06));
}
