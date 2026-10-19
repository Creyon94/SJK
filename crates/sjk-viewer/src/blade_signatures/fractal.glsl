// Fractal: a thin iridescent blade that keeps growing fractals. Glowing fractal trees branch off it
// (each limb splitting in two again and again), grow out, shimmer through the rainbow and dissolve;
// Koch-snowflake outlines spin round it, shrinking down into the blade as if zooming forever.
// A swing grows a burst of branches; a clash fires a ring of snowflakes outward.
// @room 8
// @palette {"corona.rim_cool":[0.30,0.10,0.60],"corona.rim_hot":[0.85,0.35,1.0],"corona.inner":[1.0,0.85,1.0],"core.white":[1.0,0.95,1.0],"core.fringe_cool":[0.40,0.50,1.0],"core.fringe_hot":[1.0,0.55,0.85],"light.color":[0.80,0.40,1.0],"trail":[0.78,0.42,1.0]}

mat2 fr_rot(float a){ float c = cos(a), s = sin(a); return mat2(c, -s, s, c); }
// a fractal tree folded from one trunk: distance to its limbs (q in tree units, trunk along +y)
float fr_tree(vec2 q, float spread, float depthF){
  float d = sdSeg(q, vec2(0.0), vec2(0.0, 1.0));
  float sc = 1.0;
  for (int i = 0; i < 6; i++){
    if (float(i) > depthF) break;
    q.y -= 1.0;
    q.x = abs(q.x);
    q = fr_rot(spread)*q;
    q *= 1.45; sc *= 1.45;
    d = min(d, sdSeg(q, vec2(0.0), vec2(0.0, 1.0))/sc);
  }
  return d;
}
// distance to a fractal snowflake outline of radius rad: a six-point star with smaller stars on its points
float fr_star(vec2 p, float rad){
  float a = atan(p.y, p.x), r = length(p);
  float edge = rad*(0.72 + 0.28*pow(abs(cos(a*3.0)), 3.0));
  return abs(r - edge);
}
float fr_koch(vec2 p, float rad){
  float d = fr_star(p, rad);
  float a = atan(p.y, p.x);
  float sector = floor(a/1.0472 + 0.5)*1.0472;
  vec2 tip = vec2(cos(sector), sin(sector))*rad*0.95;
  d = min(d, fr_star(fr_rot(0.5236)*(p - tip), rad*0.32));
  return d;
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float on = smoothstep(-0.5, 0.3, b.y);
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  // the blade: white core, iridescent rainbow sheath shifting along it
  vec3 iri = hsv(fract(b.y*0.04 - uTime*0.15), 0.6, 1.0);
  vec3 light = vec3(1.0, 0.97, 1.0)*exp(-pow(d/0.3, 2.0))*1.5*on + iri*halo(d, 0.8)*0.6*on + iri*halo(d, 2.0)*0.08*on;
  // fractal trees growing off the blade, both sides, growing then dissolving
  for (int i = 0; i < 5; i++){
    float fi = float(i);
    float ph = uTime*(0.22 + 0.06*h11(fi*3.3)) + h11(fi*8.1);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*2.7 + fi) < 0.5 ? -1.0 : 1.0;
    vec2 root = vec2(s*0.35, (0.1 + 0.75*h11(cy*1.9 + fi*3.0))*L);
    float size = 1.3 + 0.7*h11(cy + fi*5.0);
    vec2 dir = normalize(vec2(s, 0.55));
    vec2 perp = vec2(-dir.y, dir.x);
    vec2 rel = b - root;
    vec2 q = vec2(dot(rel, perp), dot(rel, dir))/size;
    float grow = smoothstep(0.0, 0.5, f)*6.0;
    float td = fr_tree(q, 0.42 + 0.1*sin(uTime + fi), grow)*size;
    float fade = (1.0 - smoothstep(0.65, 1.0, f));
    vec3 tc = hsv(fract(h11(cy + fi) + length(q)*0.2 - uTime*0.2), 0.7, 1.0);
    light += tc*(exp(-pow(td/0.035, 2.0))*1.4 + halo(td, 0.2)*0.18)*fade;
  }
  // snowflake outlines spinning round the blade, shrinking down into it (an endless zoom)
  for (int i = 0; i < 4; i++){
    float fi = float(i);
    float f = fract(uTime*0.18 + fi/4.0);
    float rad = 4.5*(1.0 - f) + 0.3;
    vec2 c = vec2(0.0, (0.2 + 0.2*fi)*L);
    vec2 q = fr_rot(uTime*0.5*(mod(fi, 2.0) < 0.5 ? 1.0 : -1.0) + fi)*(b - c);
    float kd = fr_koch(q, rad);
    vec3 kc = hsv(fract(fi*0.25 + uTime*0.1), 0.55, 1.0);
    light += kc*(exp(-pow(kd/0.04, 2.0))*1.1 + halo(kd, 0.2)*0.12)*smoothstep(0.0, 0.15, f)*(1.0 - smoothstep(0.85, 1.0, f));
  }
  // clash: a ring of snowflakes bursting out
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 o = vec2(0.0, L*0.6);
    for (int i = 0; i < 8; i++){
      float fi = float(i); float an = fi*0.785;
      vec2 p = o + vec2(cos(an), sin(an))*(1.0 + 6.5*(1.0 - exp(-3.0*a)));
      float kd = fr_koch(fr_rot(uTime*3.0)*(b - p), 0.6);
      light += hsv(fi*0.125, 0.6, 1.0)*(exp(-pow(kd/0.04, 2.0))*1.5)*cl;
    }
    light += vec3(1.0)*halo(length(b - o), 1.5)*pow(cl, 3.0);
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
