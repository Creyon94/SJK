// Obsidian Edge: Creyon's Blade Lab design, reworked in full. A flat blade of black volcanic glass,
// as thin as a stock blade, with glassy facets catching the light, white-hot edges crackling along
// both sides and lava veins pulsing up through the glass. Glowing chips flake off the edges and
// cool to black as they fly. A swing throws a shower of incandescent chips; a clash shatters a
// burst of shards off the blade with a white flash; ignition: the glass is forged hot from the hilt.
// @full 1
// @room 6
// @solid 1

float ob_w(float y){                      // half width of the flat glass along the blade
  float L = uLength;
  return 0.56*(1.0 - smoothstep(L - 2.2, L + 0.05, y)) + 0.05;
}
// the edge's crackle: bright flickering segments
float ob_crack(float y, float t){
  float n = vn(vec2(y*1.7, t*7.0))*0.6 + vn(vec2(y*5.0, t*13.0))*0.4;
  return 0.55 + 0.9*smoothstep(0.45, 0.85, n);
}
// lava veins: thin branching cracks drifting up through the glass
float ob_vein(vec2 b, float t){
  vec2 q = vec2(b.x*3.2, b.y*0.55 - t*0.9);
  float n = vn(q*1.7);
  float line = abs(n - 0.5);
  return smoothstep(0.06, 0.0, line)*smoothstep(0.35, 0.7, vn(vec2(b.y*0.25 - t*0.6, 3.0)));
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float on = smoothstep(-0.6, 0.3, b.y);
  float edge = 1.0 - smoothstep(6.5, 8.5, abs(b.x));
  float forge = 1.0 - smoothstep(0.0, 1.8, uAge);                    // ignition heat
  float w = ob_w(b.y);
  float ax = abs(b.x);
  float inside = step(b.y, L + 0.05)*step(-0.3, b.y);
  // white-hot edges: thin bright lines on both sides, crackling, with a hot orange bloom
  float de = abs(ax - w);
  float cr = ob_crack(b.y + sign(b.x)*7.0, uTime);
  vec3 light = vec3(1.0, 0.97, 0.92)*exp(-pow(de/0.07, 2.0))*1.6*cr*inside;
  light += vec3(1.0, 0.5, 0.15)*halo(de, 0.35)*0.5*cr*inside + vec3(1.0, 0.35, 0.08)*halo(de, 1.4)*0.12*inside;
  // the tip: the two edges meet in a hot point
  float dt = length(vec2(b.x, max(b.y - L, 0.0)));
  light += vec3(1.0, 0.8, 0.5)*halo(dt, 0.25)*0.5*step(L - 0.5, b.y);
  // sparks running along the edge
  for (int k = 0; k < 5; k++){
    float fk = float(k);
    float y = mod(uTime*(9.0 + 3.0*fk) + h11(fk*4.0)*L, L + 4.0) - 2.0;
    float s = mod(fk, 2.0) < 0.5 ? 1.0 : -1.0;
    light += vec3(1.0, 0.9, 0.7)*halo(length(vec2(b.x - s*ob_w(y), b.y - y)), 0.12)*1.5;
  }
  // chips flaking off the edges: glowing hot, cooling to dull red then dark as they fly
  for (int i = 0; i < 16; i++){
    float fi = float(i);
    float rate = 0.45 + 0.3*h11(fi*2.1);
    float ph = uTime*rate + h11(fi*7.7);
    float cy = floor(ph), f = fract(ph);
    if (h11(cy*3.1 + fi) > 0.75 + 0.25*max(uSwing, forge)) continue;
    float s = h11(cy*5.7 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.08 + 0.85*h11(cy*1.3 + fi*2.0))*L;
    vec2 v = vec2(s*(4.0 + 4.0*h11(cy + fi*9.0))*(1.0 + 0.6*uSwing), 1.5*(h11(cy*7.0 + fi) - 0.3));
    vec2 p = vec2(s*ob_w(y0), y0) + v*f - vec2(0.0, 1.8*f*f);
    vec2 q = b - p;
    float an = uTime*(5.0 + 4.0*h11(fi)) + fi;
    q = vec2(q.x*cos(an) - q.y*sin(an), q.x*sin(an) + q.y*cos(an));
    float sz = 0.32 + 0.3*h11(fi*3.3);
    float tri = max(abs(q.x)*0.87 + q.y*0.5, -q.y) - sz*0.5;     // a small triangular flake
    float heat = 1.0 - smoothstep(0.0, 0.7, f);
    vec3 hc = mix(vec3(0.5, 0.06, 0.02), mix(vec3(1.0, 0.45, 0.1), vec3(1.0, 0.95, 0.85), smoothstep(0.6, 1.0, heat)), smoothstep(0.0, 0.5, heat));
    light += hc*(fillD(tri, px)*1.3 + halo(max(tri, 0.0), 0.18)*0.5*heat)*(1.0 - smoothstep(0.75, 1.0, f));
    // a short glowing streak behind the hot ones
    vec2 pb = p - normalize(v)*2.2*heat;
    light += hc*halo(sdSeg(b, pb, p), 0.05)*heat*0.8;
  }
  // clash: a burst of big shards blown off the strike point with a white flash
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    vec2 o = vec2(0.0, L*0.55);
    light += vec3(1.0, 0.95, 0.85)*halo(length(b - o), 1.2)*pow(uClash, 4.0)*1.5;
    for (int k = 0; k < 10; k++){
      float fk = float(k);
      float an = fk*0.628 + 0.4*h11(fk*2.0);
      vec2 dir = vec2(cos(an), sin(an));
      vec2 p = o + dir*(0.5 + 7.0*(1.0 - exp(-3.0*a)));
      vec2 q = b - p;
      float rr = uTime*6.0 + fk;
      q = vec2(q.x*cos(rr) - q.y*sin(rr), q.x*sin(rr) + q.y*cos(rr));
      float sh = max(abs(q.x) - 0.12, abs(q.y) - 0.38 + abs(q.x)*1.6);
      float heat = uClash;
      light += mix(vec3(0.6, 0.1, 0.03), vec3(1.0, 0.85, 0.6), heat)*(fillD(sh, px) + halo(max(sh, 0.0), 0.25)*0.5);
    }
  }
  light *= on*edge*(1.0 - smoothstep(L + 1.5, L + 5.0, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
// the glass itself: black, faceted, with moving glints and the lava veins inside
vec4 sig_solid(vec4 o, vec2 b, float px){
  px = min(px, 0.06);
  float L = uLength;
  float w = ob_w(b.y);
  float cov = fillD(abs(b.x) - w, px)*step(-0.3, b.y)*fillD(b.y - L - 0.05, px);
  if (cov <= 0.0) return o;
  float forge = 1.0 - smoothstep(0.0, 1.8, uAge);
  // facets: conchoidal ripples in the glass, a ridge down the middle
  float ridge = 1.0 - abs(b.x)/max(w, 0.01);
  float conch = vn(vec2(b.x*4.0, b.y*0.9))*0.6 + vn(vec2(b.x*9.0, b.y*2.3))*0.4;
  float rings = 0.5 + 0.5*sin(conch*14.0);
  // a glint sweeping up the blade, and a cool sky sheen on one face
  float gl = exp(-pow((b.y - mod(uTime*11.0, L + 16.0) + 8.0 + b.x*3.0)/1.2, 2.0));
  vec3 glass = vec3(0.025, 0.022, 0.03) + vec3(0.06, 0.07, 0.1)*rings*0.6 + vec3(0.12, 0.13, 0.17)*smoothstep(0.7, 1.0, ridge)*0.6;
  glass += vec3(0.75, 0.8, 0.95)*gl*(0.25 + 0.5*rings)*smoothstep(0.1, 0.9, ridge);
  // lava veins glowing inside, brighter while forging and on swing
  float vein = ob_vein(b, uTime);
  vec3 lava = mix(vec3(0.9, 0.2, 0.03), vec3(1.0, 0.75, 0.3), vein);
  glass = mix(glass, lava*(1.2 + 0.8*uSwing), vein*(0.7 + 0.3*sin(uTime*3.0 + b.y*0.4)));
  glass = mix(glass, vec3(1.0, 0.45, 0.1), forge*0.85);
  return vec4(mix(o.rgb, glass, cov), max(o.a, cov*0.98*clamp(uIgn*1.5, 0.0, 1.0)));
}
