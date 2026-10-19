// Beskar Forge: a blade of Mandalorian steel fresh from the forge. A bright silver blade with an
// orange forge heat glowing up from the hilt and cooling to steel-blue toward the tip; molten
// beskar beads along it and drips off; in a slow rhythm the armorer's hammer strikes, a band of
// white heat flares where it lands and sparks bounce away and fall; when the heat passes, a
// hiss of quench steam rises. A swing throws sparks off it; a clash is a hammer strike.
// @part Forge sparks
// @room 7
// @palette {"corona.rim_cool":[0.30,0.36,0.45],"corona.rim_hot":[0.80,0.86,0.95],"corona.inner":[0.95,0.97,1.0],"core.white":[1.0,1.0,1.0],"core.fringe_cool":[0.65,0.72,0.85],"core.fringe_hot":[0.92,0.95,1.0],"light.color":[0.85,0.88,0.95],"trail":[0.80,0.85,0.95]}

const float BF_P = 2.2;
float bf_top(){ return uLength*clamp(uAge/0.45, 0.0, 1.0); }
float bf_y(float n){ return uLength*(0.2 + 0.6*h11(n*2.71 + 0.4)); }
vec3 bf_heat(float h){
  vec3 c = mix(vec3(0.6, 0.1, 0.0), vec3(1.0, 0.45, 0.08), smoothstep(0.0, 0.5, h));
  return mix(c, vec3(1.0, 0.92, 0.75), smoothstep(0.5, 1.0, h));
}
vec3 base_vol(vec3 ro, vec3 rd, float px){
  float t = uTime, top = bf_top(), s, ta;
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 col = vec3(0.0);
  if (top < 0.5) return col;
  float n = floor(t/BF_P), ph = fract(t/BF_P)*BF_P;
  float ys = bf_y(n);
  float strike = max(exp(-ph*3.0), cl);
  // sparks bouncing away from the strike and falling
  if (ys < top){
    for (int i = 0; i < 20; i++){
      float fi = float(i);
      float a = h11(fi*3.1 + n);
      vec3 dir = normalize(vec3(sin(6.2832*a), 0.3 + 0.8*h11(fi*5.3 + n), cos(6.2832*a)));
      float v = 4.0 + 5.0*h11(fi*1.7 + n);
      float tt = ph*0.9, tq = max(tt - 0.04, 0.0);
      vec3 p1 = vec3(0.0, ys, 0.0) + dir*v*tt + vec3(0.0, -9.0*tt*tt, 0.0);
      vec3 p0 = vec3(0.0, ys, 0.0) + dir*v*tq + vec3(0.0, -9.0*tq*tq, 0.0);
      float d = raySeg(ro, rd, p0, p1, s);
      col += bf_heat(1.0 - ph*0.6)*(halo(d, max(0.04, px))*2.0 + halo(d, 0.2)*0.2)*(1.0 - smoothstep(0.6, 1.4, ph))*(0.7 + 0.6*sw);
    }
    // the flash where the hammer lands
    float dk = rayPt(ro, rd, vec3(0.0, ys, 0.0), ta);
    col += vec3(1.0, 0.9, 0.75)*(halo(dk, 0.7)*2.0 + halo(dk, 2.0)*0.4)*strike;
  }
  // molten beskar beading along the blade and dripping off
  for (int i = 0; i < 8; i++){
    float fi = float(i);
    float f = fract(t*(0.25 + 0.1*h11(fi*2.2)) + h11(fi*6.6));
    float k = floor(t*(0.25 + 0.1*h11(fi*2.2)) + h11(fi*6.6));
    float y0 = (0.1 + 0.6*h11(fi*3.3 + k))*top;
    vec3 p = around(0.45 + 0.4*max(f - 0.5, 0.0), h11(fi*9.9 + k), y0 - 5.0*max(f - 0.4, 0.0)*max(f - 0.4, 0.0));
    float d = rayPt(ro, rd, p, ta);
    col += bf_heat(0.8 - 0.4*f)*(halo(d, max(0.09, px))*1.6 + halo(d, 0.4)*0.2)*sin(3.1416*f);
  }
  // quench steam rising off it (round 12: soft smoke, not puffs) and forge embers drifting up
  col += fxSmoke(ro, rd, top*0.2, top*0.95, 2.6, 2.5, 5.5, vec3(0.75, 0.78, 0.85))*0.55;
  col += fxMotes(ro, rd, px, top*0.2, top*0.9, 0.7, 3.0, 14.0, 0.06, 2.6, vec3(1.0, 0.55, 0.2))*0.8;
  // a cold steel sheen sliding up its upper half
  col += fxStreams(ro, rd, px, top*0.5, top*0.95, 1.0, 1.4, 0.3, 1.2, 8.1, vec3(0.75, 0.82, 0.95))*0.5;
  return col*uFx;
}
// the blade: Sol's glow and core from the file, forge heat glowing up from the hilt and cooling
// toward the tip, a white-hot band where the hammer lands
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = floor(uTime/BF_P), ph = fract(uTime/BF_P)*BF_P;
  float band = halo(b.y - bf_y(n), 1.2)*max(exp(-ph*2.0), clamp(uClash, 0.0, 1.0));
  float heat = (1.0 - smoothstep(0.0, uLength*0.55, b.y))*(0.6 + 0.2*sin(uTime*1.5 + b.y*0.3));
  float l = max(g.r, max(g.g, g.b));
  vec3 steel = g*(0.9 + 0.15*halo(b.y - mod(uTime*8.0, uLength + 20.0) + 10.0, 2.0));
  return mix(steel, l*bf_heat(heat + band), clamp(heat + band, 0.0, 1.0)) + vec3(1.0, 0.9, 0.75)*band*l*1.2;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// round 11: sig_vol gated by part 0
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  if (part(0.0) < 0.5) { return vec3(0.0); }
  return base_vol(ro, rd, px);
}
