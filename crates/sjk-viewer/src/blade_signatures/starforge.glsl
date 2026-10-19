// Starforge: a kyber blade forged inside a star. Thin white-hot steel cooling to orange at its edges,
// a rounded point. Star-fire prominences arch off it and erupt away; in a steady rhythm the forge
// strikes: a white flash where it lands, a big fountain of sparks that arc and fall, a shock wave,
// and a white-hot spot that cools back to orange. Embers drift up all round. A swing strikes faster;
// a clash is a huge blow with a ring of sparks.
// @room 8
// @palette {"corona.rim_cool":[0.55,0.12,0.02],"corona.rim_hot":[1.0,0.55,0.12],"corona.inner":[1.0,0.88,0.55],"core.white":[1.0,0.96,0.85],"core.fringe_cool":[1.0,0.40,0.08],"core.fringe_hot":[1.0,0.80,0.45],"light.color":[1.0,0.58,0.18],"trail":[1.0,0.55,0.15]}

// A steady rhythm: a period that changed with the swing jumped the phase by uTime times the
// change, so the forge raced in game (whose clock runs on for minutes).
float sf_P(){ return 0.9; }
// the current blow: phase (0 at impact), height along the blade
float sf_ph(){ return fract(uTime/sf_P()); }
float sf_y(float n){ return uLength*(0.25 + 0.55*h11(n*3.7)); }
float sf_n(){ return floor(uTime/sf_P()); }
// the hammer, drawn in its own frame: a head (box) on a handle, pivoting at the handle's end
float sf_hammer(vec2 q){
  float head = sdBox(q - vec2(0.0, 3.2), vec2(1.0, 0.55));
  float handle = sdBox(q - vec2(0.0, 1.5), vec2(0.13, 1.5));
  return min(head, handle);
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float cl = clamp(uClash, 0.0, 1.0);
  vec3 hot = vec3(1.0, 0.55, 0.12), white = vec3(1.0, 0.95, 0.85);
  float on = smoothstep(-0.5, 0.3, b.y);
  // the blade: rounded point, white-hot core, orange edges
  float yc = clamp(b.y, 0.0, L - 0.5);
  float d = length(vec2(b.x, b.y - yc));
  float ph = sf_ph(), n = sf_n();
  float y0 = sf_y(n), y1 = sf_y(n - 1.0);
  // the spots where the last blows landed, cooling
  float spot = exp(-pow((b.y - y0)/1.2, 2.0))*exp(-ph*2.0) + exp(-pow((b.y - y1)/1.2, 2.0))*exp(-(ph + 1.0)*2.0);
  vec3 steel = mix(hot, white, clamp(0.55 + spot*1.5, 0.0, 1.0));
  vec3 light = steel*exp(-pow(d/0.34, 2.0))*1.5*on + hot*halo(d, 0.9)*(0.55 + 0.6*spot)*on + vec3(1.0, 0.35, 0.05)*halo(d, 2.2)*0.1*on;
  // star-fire licking off the blade: flame tongues curling up and away on both sides
  for (int i = 0; i < 10; i++){
    float fi = float(i);
    float pph = uTime*(0.55 + 0.3*h11(fi*3.3)) + h11(fi*8.1);
    float cy = floor(pph), f = fract(pph);
    float s = h11(cy*2.7 + fi) < 0.5 ? -1.0 : 1.0;
    float yy = (0.06 + 0.85*h11(cy*1.9 + fi*3.0))*L;
    float len = (4.0 + 3.5*h11(cy + fi))*smoothstep(0.0, 0.3, f);
    float u = (b.x*s - 0.35)/max(len, 0.01);
    if (u < 0.0 || u > 1.0) continue;
    float dy = b.y - yy - 2.4*u*u - 0.5*sin(u*7.0 - uTime*12.0 + fi)*u;
    float th = 0.5*(1.0 - u) + 0.05;
    float fl = exp(-pow(dy/th, 2.0))*(1.0 - u)*(1.0 - f)*smoothstep(0.0, 0.08, f);
    light += mix(white, vec3(1.0, 0.3, 0.03), smoothstep(0.0, 0.8, u))*fl*2.2;
  }
  // the blow: a white flash, a shock ring and a fountain of sparks
  float k = exp(-ph*9.0);
  vec2 o = vec2(0.3, y0);
  light += white*halo(length(b - o), 0.3 + 0.7*k)*k*1.6;
  float rr = abs(length((b - o)*vec2(1.0, 1.3)) - 0.5 - 6.0*ph);
  light += hot*halo(rr, 0.08)*(1.0 - ph)*smoothstep(0.0, 0.05, ph)*1.2;
  for (int i = 0; i < 18; i++){
    float fi = float(i);
    float an = -0.6 + 2.6*h11(fi*3.1 + n);                          // spray mostly up and to the left
    vec2 v = vec2(-cos(an)*(3.0 + 5.0*h11(fi*7.3)), sin(an)*(3.0 + 5.0*h11(fi*1.7)) + 2.0);
    float t = ph*sf_P()*1.6;
    vec2 p = o + v*t - vec2(0.0, 6.0*t*t);
    vec2 pv = o + v*max(t - 0.06, 0.0) - vec2(0.0, 6.0*max(t - 0.06, 0.0)*max(t - 0.06, 0.0));
    float life = 1.0 - smoothstep(0.4, 1.0, ph);
    light += mix(white, hot, ph)*(halo(sdSeg(b, pv, p), 0.07)*2.2 + halo(length(b - p), 0.18)*0.9)*life;
  }
  // embers drifting up all round
  for (int i = 0; i < 12; i++){
    float fi = float(i);
    float f = fract(uTime*(0.15 + 0.08*h11(fi*2.9)) + h11(fi*5.1));
    vec2 p = vec2((h11(fi*3.7) - 0.5)*10.0 + 0.6*sin(uTime + fi), f*(L + 4.0) - 2.0);
    light += hot*halo(length(b - p), 0.07)*1.5*(0.5 + 0.5*sin(uTime*5.0 + fi))*smoothstep(0.0, 0.1, f)*(1.0 - smoothstep(0.8, 1.0, f));
  }
  // clash: a huge blow, a ring of sparks
  if (cl > 0.01){
    float a = 1.0 - cl;
    vec2 c = vec2(0.0, L*0.6);
    light += white*halo(length(b - c), 1.8)*pow(cl, 3.0)*2.0;
    for (int i = 0; i < 14; i++){
      float fi = float(i); float an = fi*0.449;
      vec2 dir = vec2(cos(an), sin(an));
      float rd = 0.6 + 7.0*(1.0 - exp(-3.0*a));
      light += mix(white, hot, a)*halo(sdSeg(b, c + dir*max(rd - 1.0, 0.0), c + dir*rd), 0.05)*cl*1.6;
    }
  }
  float edge = 1.0 - smoothstep(8.5, 10.2, abs(b.x));
  light *= edge*(1.0 - smoothstep(L + 0.5, L + 3.5, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.15))*1.05;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
