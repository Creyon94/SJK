// Death Star Plans: the stolen battle station plans as a blade. A cool blueprint-blue blade with
// a bright scan line racing up it; round it, a holographic schematic turns slowly in 3D: thin
// rings of the station's trenches and decks circling it at different heights, a reactor shaft
// line and cross beams, and points of data blinking along them as the plans are read. Now and
// then the schematic flickers like a damaged hologram. A swing makes the projection jitter;
// a clash flashes it and blows the rings outward.
// @room 6
// @palette {"corona.rim_cool":[0.04,0.20,0.50],"corona.rim_hot":[0.30,0.65,1.0],"corona.inner":[0.75,0.90,1.0],"core.white":[0.92,0.97,1.0],"core.fringe_cool":[0.25,0.55,1.0],"core.fringe_hot":[0.70,0.88,1.0],"light.color":[0.35,0.65,1.0],"trail":[0.40,0.70,1.0]}

float dp_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
float dp_ring(vec3 ro, vec3 rd, float y, float R){
  float best = 1e3;
  vec3 h = rayAt(ro, rd, y);
  if (h.z > 0.0) best = abs(length(h.xy) - R);
  float a = dot(rd.xz, rd.xz), b = dot(ro.xz, rd.xz), c = dot(ro.xz, ro.xz) - R*R;
  float dsc = b*b - a*c;
  if (dsc > 0.0 && a > 1e-6){
    float q = sqrt(dsc);
    float t0 = (-b - q)/a, t1 = (-b + q)/a;
    if (t0 > 0.0) best = min(best, abs(ro.y + rd.y*t0 - y));
    if (t1 > 0.0) best = min(best, abs(ro.y + rd.y*t1 - y));
  }
  return best;
}
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float t = uTime, top = dp_top(), s, ta;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 blue = vec3(0.35, 0.7, 1.0), pale = vec3(0.8, 0.93, 1.0);
  vec3 col = vec3(0.0);
  if (top < 0.5) return col;
  float flick = 0.75 + 0.25*step(0.15, h11(floor(t*12.0)))*1.0 - 0.5*step(0.93, h11(floor(t*5.0) + 3.0));
  float jit = 0.08*sw*(h11(floor(t*30.0)) - 0.5);
  vec3 o = vec3(jit, 0.0, 0.0);
  // deck rings at different heights and sizes, turning, data points blinking along them
  for (int i = 0; i < 6; i++){
    float fi = float(i);
    float y = (0.12 + 0.15*fi)*uLength;
    if (y > top) continue;
    float R = (1.2 + 1.6*h11(fi*3.7))*(1.0 + 2.5*cl*(1.0 - cl));
    float dr = dp_ring(ro - o, rd, y, R);
    col += (blue*halo(dr, max(0.03, px))*1.0 + blue*halo(dr, 0.2)*0.12);
    for (int j = 0; j < 4; j++){
      float a = h11(fi*5.0 + float(j)) + t*(0.05 + 0.03*fi)*(mod(fi, 2.0) < 0.5 ? 1.0 : -1.0);
      float dk = rayPt(ro - o, rd, around(R, a, y), ta);
      float bl = step(0.5, fract(t*1.5 + fi*0.3 + float(j)*0.27));
      col += pale*halo(dk, max(0.06, px))*1.5*bl;
    }
  }
  // the reactor shaft and cross beams, slowly turning
  float rot0 = t*0.08;
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float y0 = (0.2 + 0.2*fi)*uLength;
    if (y0 > top) continue;
    float a = rot0 + fi*0.125;
    float d = raySeg(ro - o, rd, around(2.4, a, y0), around(2.4, a + 0.5, y0), s);
    col += blue*halo(d, max(0.025, px))*0.7*(0.5 + 0.5*sin(s*20.0 - t*4.0));
  }
  // the scan line sweeping a band of light round the schematic
  float ys = mod(t*12.0, top + 6.0) - 3.0;
  float dsc = dp_ring(ro - o, rd, ys, 2.0);
  col += pale*halo(dsc, 0.08 + px)*1.2 + blue*halo(dsc, 0.6)*0.2;
  if (cl > 0.01){
    float dc = rayPt(ro, rd, vec3(0.0, uLength*0.6, 0.0), ta);
    col += pale*halo(dc, 2.0)*cl*cl*1.5;
  }
  return col*flick*uFx;
}
// the blade: Sol's glow and core from the file, a bright scan line racing up it
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float ys = mod(uTime*12.0, uLength + 6.0) - 3.0;
  float ln = 0.85 + 0.15*step(0.5, fract(b.y*2.0 - uTime*3.0));
  return g*ln + vec3(0.8, 0.93, 1.0)*halo(b.y - ys, 0.25)*smoothstep(1.5, 0.0, abs(b.x))*1.4;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
