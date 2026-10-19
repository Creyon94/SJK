// Event Horizon: the blade is a gravity well. A thin violet-white blade with a dark heart, and round
// it spacetime itself bends: a faint grid of light curving in toward the blade, stars falling in on
// spiral paths and stretching into streaks (spaghettified) as they reach it, a hot accretion swirl
// hugging the blade, and gravitational waves rippling outward up its length. A swing drags the grid
// and pulls in a storm of stars; a clash fires a gravity wave and a photon-ring flash.
// @room 8
// @palette {"corona.rim_cool":[0.30,0.06,0.70],"corona.rim_hot":[0.70,0.40,1.0],"corona.inner":[0.9,0.85,1.0],"core.white":[0.95,0.9,1.0],"core.fringe_cool":[0.40,0.12,0.90],"core.fringe_hot":[0.80,0.55,1.0],"light.color":[0.55,0.25,1.0],"trail":[0.60,0.35,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  vec3 vio = vec3(0.6, 0.35, 1.0), white = vec3(0.95, 0.92, 1.0), hot = vec3(1.0, 0.65, 0.4);
  float on = smoothstep(-0.5, 0.3, b.y);
  float yc = clamp(b.y, 0.0, L - 0.35);
  float d = length(vec2(b.x, b.y - yc));
  float ax = abs(b.x);
  // the blade: a dark heart inside a razor-thin bright photon ring on each side, a violet glow
  float ring = exp(-pow((d - 0.32)/0.07, 2.0));
  vec3 light = white*ring*1.6*on + vio*halo(d, 0.9)*0.5*on + vio*halo(d, 2.2)*0.08*on;
  light *= 1.0 - 0.9*fillD(d - 0.22, px)*on;
  // two spiral arms of glowing gas winding in toward the blade (a whirlpool round a black hole)
  float gfade = smoothstep(0.0, 3.0, b.y)*(1.0 - smoothstep(L - 2.0, L + 1.0, b.y));
  for (int k = 0; k < 2; k++){
    float fk = float(k);
    float yc0 = L*(0.35 + 0.3*fk) + 1.5*sin(uTime*0.3 + fk);
    vec2 q = vec2(b.x, (b.y - yc0)/0.45);
    float rr = length(q), aa = atan(q.y, q.x);
    float arm = sin(aa*2.0 + log(max(rr, 0.15))*5.0 - uTime*(2.0) + fk*1.5);
    float band = smoothstep(0.55, 1.0, arm)*smoothstep(7.0, 2.0, rr)*smoothstep(0.4, 1.2, rr);
    light += mix(vio, hot, smoothstep(5.0, 1.0, rr))*band*0.55;
    // lensed starlight: thin bright arcs hugging the blade
    light += white*halo(abs(rr - 2.2 - 0.3*sin(uTime + fk)), 0.03)*smoothstep(0.3, 0.9, abs(sin(aa + uTime*0.7)))*0.6;
  }
  // stars falling in on spirals, stretched into streaks as they near the blade
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float ph = uTime*(0.3 + 0.15*h11(fi*2.3)) + h11(fi*7.7);
    float cy = floor(ph), f = fract(ph);
    float s = h11(cy*3.1 + fi) < 0.5 ? -1.0 : 1.0;
    float y0 = (0.1 + 0.8*h11(cy*1.9 + fi*3.0))*L;
    float rad = 7.5*(1.0 - f*f) + 0.3;
    float x = s*rad;
    float y = y0 + (1.0 - f)*2.5*sin(f*6.0 + fi);
    vec2 p = vec2(x, y);
    float stretch = 1.0 + 6.0*f*f*f;                               // spaghettified near the blade
    vec2 q = b - p;
    float dd = length(vec2(q.x, q.y/stretch));
    float k = smoothstep(0.0, 0.15, f)*(1.0 - smoothstep(0.92, 1.0, f));
    light += mix(white, hot, f)*(halo(dd, 0.07 + 0.04*f)*1.8 + halo(dd, 0.3)*0.15)*k;
  }
  // the accretion swirl: hot matter streaks spiralling tight round the blade, brighter where they cross in front
  for (int k = 0; k < 3; k++){
    float fk = float(k);
    float ph = b.y*0.7 - uTime*(5.0 + fk) + fk*2.1;
    float x = (0.75 + 0.35*fk)*sin(ph);
    float front = 0.5 + 0.5*cos(ph);
    light += mix(hot, white, front)*halo(abs(b.x - x), 0.05)*(0.25 + 0.9*front)*on*step(b.y, L - 0.6)*0.9;
  }
  // gravitational waves rippling outward from the blade, travelling up it
  float wv = sin(ax*2.2 - uTime*4.0 + b.y*0.3);
  light += vio*halo(abs(wv), 0.08)*exp(-ax/3.0)*smoothstep(0.4, 1.2, ax)*0.35*(1.0 + 2.0*cl)*gfade;
  // clash: a gravity wave ring and a photon-ring flash
  if (cl > 0.01){
    vec2 o = vec2(0.0, L*0.6);
    float rr = abs(length((b - o)*vec2(1.0, 0.7)) - 0.8 - 7.0*(1.0 - cl));
    light += white*(halo(rr, 0.1)*1.6 + halo(rr, 0.5)*0.3)*cl + white*ring*cl*1.5;
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, ax);
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
