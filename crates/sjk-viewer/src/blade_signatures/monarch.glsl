// Monarch: Creyon's Blade Lab design, reworked with its butterflies drawn in full. A thin warm amber
// blade with real monarch butterflies (orange wings, black veins, white-spotted borders) resting on
// it, taking off in looping flights trailed by golden pollen and landing again. A swing scatters
// them all; a clash blows them off in a burst of pollen; every few seconds a swarm of glowing
// butterflies spirals up the blade and bursts off the tip. Ignition: they fly in from all round.
// @full 1
// @room 7
// @solid 1

const int MN_N = 10;
// position of butterfly i at time t (SJK units): rests on the blade's edge, flies loops off it
vec2 mn_pos(float fi, float t){
  float L = uLength;
  float side = mod(fi, 2.0) < 0.5 ? 1.0 : -1.0;
  float yh = (0.12 + 0.8*fract(fi*0.618 + 0.13))*L;
  vec2 home = vec2(side*0.9, yh);
  float rate = 0.13 + 0.05*h11(fi*3.7);
  float c = fract(t*rate + h11(fi*9.1));
  float fly = smoothstep(0.0, 0.1, c)*(1.0 - smoothstep(0.42, 0.56, c));
  float a = c*6.2832*1.3 + fi;
  vec2 loop = vec2(side*(2.6 + 2.2*sin(a)), 2.6*sin(a*0.8) + 1.5*sin(a*1.9));
  vec2 p = home + loop*fly;
  // swing / clash: everyone scatters outward and up, then drifts back
  float sc = max(uSwing, uClash*1.3);
  float da = 6.2832*h11(fi*5.3);
  vec2 dir = normalize(vec2(side*(1.2 + abs(cos(da))), sin(da) + 0.4));
  p += dir*(4.5 + 2.0*h11(fi*2.9))*smoothstep(0.0, 1.0, sc);
  // ignition: they arrive from far off
  float arr = smoothstep(0.0, 2.2, uAge - 0.15*fi);
  p += dir*9.0*(1.0 - arr);
  p.y = clamp(p.y, -1.0, L + 3.0);
  return p;
}
// how much butterfly i is flying (0 resting .. 1 in the air)
float mn_air(float fi, float t){
  float c = fract(t*(0.13 + 0.05*h11(fi*3.7)) + h11(fi*9.1));
  return max(smoothstep(0.0, 0.1, c)*(1.0 - smoothstep(0.42, 0.56, c)), max(uSwing, uClash));
}
// a monarch in its own frame: q.y along the body (head +), q.x across; s = size, open = wing opening.
// returns (wing orange amount, black, white spot, body) coverage
vec4 mn_shape(vec2 q, float s, float open, float px){
  float w = abs(q.x)/max(open, 0.12);
  float u = q.y;
  vec2 f = vec2(w - 0.52*s, u - 0.2*s);
  f = vec2(f.x*0.94 - f.y*0.34, f.x*0.34 + f.y*0.94);
  float fore = length(f/vec2(0.56*s, 0.36*s)) - 1.0;
  vec2 h = vec2(w - 0.38*s, u + 0.26*s);
  float hind = length(h/vec2(0.38*s, 0.31*s)) - 1.0;
  float wing = min(fore, hind);
  float ed = px*1.5/(s*0.3*max(open, 0.12));
  float cov = clamp(0.5 - wing/max(ed, 1e-3), 0.0, 1.0)*step(0.02, w + 0.02);
  // black border band, black veins radiating from the root
  float border = smoothstep(-0.36, -0.24, wing);
  float ang = atan(u + 0.05*s, w + 0.02);
  float veins = smoothstep(0.78, 0.93, abs(sin(ang*4.5 + w*2.0/s)))*smoothstep(0.08*s, 0.25*s, w);
  float black = max(border, veins*0.85);
  // white spots strung along the border
  float sa = atan(f.y, f.x)*7.0, sb = atan(h.y, h.x)*6.0;
  float spot = max(smoothstep(0.45, 0.8, cos(sa))*smoothstep(-0.2, -0.12, fore)*(1.0 - smoothstep(-0.08, -0.02, fore)),
                   smoothstep(0.5, 0.85, cos(sb))*smoothstep(-0.2, -0.12, hind)*(1.0 - smoothstep(-0.08, -0.02, hind)));
  float body = clamp(0.5 - (length(vec2(q.x*2.6, clamp(u, -0.42*s, 0.42*s) - u)) - 0.12*s)/px, 0.0, 1.0)
             * step(-0.5*s, u)*step(u, 0.5*s) + clamp(0.5 - (abs(q.x) - 0.05*s)/px, 0.0, 1.0)*step(-0.42*s, u)*step(u, 0.42*s);
  return vec4(cov, black*cov, spot*cov, clamp(body, 0.0, 1.0));
}
// local frame of butterfly i: rotate by its heading
vec2 mn_frame(vec2 b, vec2 p, vec2 v){
  vec2 hd = length(v) > 1e-3 ? normalize(v) : vec2(0.0, 1.0);
  vec2 d = b - p;
  return vec2(d.x*hd.y - d.y*hd.x, d.x*hd.x + d.y*hd.y);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float on = smoothstep(-0.6, 0.3, b.y);
  float edge = 1.0 - smoothstep(7.5, 10.0, abs(b.x));
  float tipF = 1.0 - smoothstep(L + 2.0, L + 4.5, b.y);
  // the blade: warm white core, amber glow, a slow honey shimmer
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  float shim = 0.9 + 0.1*sin(b.y*0.6 - uTime*2.2);
  vec3 light = vec3(1.0, 0.95, 0.82)*exp(-pow(d/0.36, 2.0))*1.5*on;
  light += vec3(1.0, 0.55, 0.12)*halo(d, 0.85)*0.6*shim*on + vec3(1.0, 0.45, 0.08)*halo(d, 2.2)*0.1*on;
  // butterflies: a warm glow round each, golden pollen trailing the flyers
  for (int i = 0; i < MN_N; i++){
    float fi = float(i);
    vec2 p = mn_pos(fi, uTime);
    float air = mn_air(fi, uTime);
    float dd = length(b - p);
    light += vec3(1.0, 0.5, 0.1)*halo(dd, 1.4)*(0.05 + 0.08*air);
    for (int k = 1; k < 5; k++){
      float fk = float(k);
      vec2 pk = mn_pos(fi, uTime - fk*0.09) + vec2(0.0, -0.15*fk) + 0.25*(vec2(h11(fi + fk*7.0), h11(fi*3.0 + fk)) - 0.5)*fk;
      light += vec3(1.0, 0.82, 0.4)*halo(length(b - pk), 0.09)*(1.1 - fk*0.22)*air*0.9;
    }
  }
  // the swarm: every 9 s glowing butterflies spiral up round the blade and burst off the tip
  float sp = mod(uTime, 9.0)/3.2;
  if (sp < 1.0){
    for (int k = 0; k < 7; k++){
      float fk = float(k);
      float pr = sp*1.35 - fk*0.06;
      if (pr < 0.0 || pr > 1.25) continue;
      float y = pr*L;
      float burst = max(pr - 0.95, 0.0)*14.0;
      float ang = pr*14.0 + fk*0.9;
      vec2 p = vec2(2.0*sin(ang) + sign(sin(fk*2.1 + 0.3))*burst, y + burst*0.6);
      vec2 v = vec2(2.0*cos(ang)*14.0, L);
      vec2 q = mn_frame(b, p, v);
      vec4 sh = mn_shape(q, 1.1, 0.25 + 0.75*abs(sin(uTime*16.0 + fk)), px);
      float fade = smoothstep(0.0, 0.08, pr)*(1.0 - smoothstep(1.0, 1.25, pr));
      float depth = 0.55 + 0.45*cos(ang);
      light += (vec3(1.0, 0.6, 0.12)*sh.x*(1.0 - sh.y) + vec3(1.0, 0.95, 0.8)*sh.z + vec3(1.0, 0.7, 0.3)*halo(length(b - p), 0.6)*0.35)*fade*depth*1.2;
      light += vec3(1.0, 0.8, 0.4)*halo(length(b - (p - normalize(v)*0.9)), 0.12)*fade*0.6;
    }
  }
  // clash: a pollen burst blown off the blade
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    for (int k = 0; k < 12; k++){
      float fk = float(k);
      float an = fk*0.5236 + 0.3*h11(fk);
      vec2 p = vec2(0.0, L*0.55) + vec2(cos(an), sin(an))*(1.0 + 7.0*a);
      light += vec3(1.0, 0.8, 0.35)*halo(length(b - p), 0.14)*uClash*1.4;
    }
    light += vec3(1.0, 0.6, 0.2)*halo(length(b - vec2(0.0, L*0.55)), 2.0)*pow(uClash, 3.0)*0.6;
  }
  light *= edge*tipF*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
// the butterflies themselves, as solid wings over the glow
vec4 sig_solid(vec4 o, vec2 b, float px){
  px = min(px, 0.06);
  float L = uLength;
  if (b.y < -3.0 || b.y > L + 6.0 || abs(b.x) > 10.0) return o;
  vec4 acc = o;
  for (int i = 0; i < MN_N; i++){
    float fi = float(i);
    vec2 p = mn_pos(fi, uTime);
    if (length(b - p) > 2.4) continue;
    float air = mn_air(fi, uTime);
    vec2 v = mn_pos(fi, uTime + 0.05) - p;
    // resting: heads point up the blade, wings slowly opening; flying: fast flaps
    v = mix(vec2(0.0, 1.0), v, smoothstep(0.1, 0.4, air));
    float open = mix(0.55 + 0.45*sin(uTime*1.6 + fi*1.7), 0.18 + 0.82*abs(sin(uTime*15.0 + fi*2.3)), smoothstep(0.1, 0.5, air));
    float s = 1.55 + 0.35*h11(fi*4.1);
    vec2 q = mn_frame(b, p, v);
    vec4 sh = mn_shape(q, s, open, px);
    float shade = 0.75 + 0.25*open;
    vec3 wingC = mix(vec3(0.95, 0.32, 0.02), vec3(1.0, 0.55, 0.08), smoothstep(0.0, 0.6*s, abs(q.x)))*shade*1.15;
    vec3 col = mix(wingC, vec3(0.03, 0.02, 0.02), sh.y);
    col = mix(col, vec3(1.0, 0.97, 0.9), sh.z);
    col = mix(col, vec3(0.04, 0.03, 0.03), sh.w);
    float a = max(sh.x, sh.w)*0.97*clamp(uIgn*1.5, 0.0, 1.0);
    acc = vec4(mix(acc.rgb, col, a), max(acc.a, a));
  }
  return acc;
}
