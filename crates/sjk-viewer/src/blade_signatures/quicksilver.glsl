// Quicksilver: Creyon's Blade Lab design, reworked in full. A thin blade of liquid mirror: a chrome
// surface reflecting a cool sky and warm floor, ripples racing up it, beads of mercury rolling up to
// the tip and dripping off in falling droplets. A swing flings a spray of silver droplets off the
// blade that arc away and splash into smaller beads; a clash throws a ring of mercury splashing out
// with a bright ripple running both ways; ignition pours the metal up from the hilt.
// @full 1
// @room 6
// @solid 1

float qs_w(float y){ float L = uLength; return 0.5*smoothstep(-0.3, 0.5, y)*(1.0 - smoothstep(L - 0.9, L + 0.05, y)) + 0.05; }
// fake chrome: what the curved mirror reflects at a slope n (-1..1 across) and height y
vec3 qs_env(float n, float y, float t){
  float h = n*0.9 + 0.12*sin(y*0.35 - t*1.3);
  vec3 sky = mix(vec3(0.55, 0.65, 0.85), vec3(0.95, 0.97, 1.0), smoothstep(-0.1, 0.6, h));
  vec3 floor_ = mix(vec3(0.12, 0.1, 0.12), vec3(0.4, 0.33, 0.3), smoothstep(-1.0, -0.3, h));
  vec3 c = mix(floor_, sky, smoothstep(-0.25, 0.05, h));
  c += vec3(1.0)*exp(-pow((h - 0.45)/0.06, 2.0))*1.2;     // the horizon highlight
  return c;
}
// a mercury drop: sphere shading with a highlight, returns (rgb, coverage)
vec4 qs_drop(vec2 q, float rad, float px){
  float d = length(q);
  float cov = fillD(d - rad, px);
  if (cov <= 0.0) return vec4(0.0);
  vec2 n = q/max(rad, 1e-3);
  vec3 c = qs_env(n.x, n.y*3.0, 0.0)*(0.65 + 0.35*sqrt(max(1.0 - dot(n, n), 0.0)));
  c += vec3(1.0)*exp(-dot(n - vec2(-0.35, 0.4), n - vec2(-0.35, 0.4))*18.0)*1.3;
  return vec4(c, cov);
}
// flung droplet k (spray off the blade): position and radius, alive amount
vec4 qs_fling(float fk, float t){
  float L = uLength;
  float rate = 0.4 + 0.25*h11(fk*2.3);
  float ph = t*rate + h11(fk*8.1);
  float cy = floor(ph), f = fract(ph);
  float live = step(h11(cy*2.9 + fk), 0.45 + 0.55*uSwing + 0.5*uClash);
  float s = h11(cy*4.1 + fk) < 0.5 ? -1.0 : 1.0;
  float y0 = (0.15 + 0.8*h11(cy*1.7 + fk*3.0))*L;
  vec2 v = vec2(s*(4.0 + 4.0*h11(cy + fk*5.0)), 2.5*(h11(cy*6.0 + fk) - 0.2));
  vec2 p = vec2(s*0.4, y0) + v*f - vec2(0.0, 3.5*f*f);
  return vec4(p, (0.3 + 0.25*h11(fk*4.4))*(1.0 - 0.4*f), live*(1.0 - smoothstep(0.8, 1.0, f)));
}
vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float L = uLength;
  px = min(px, 0.06);
  float on = smoothstep(-0.6, 0.3, b.y);
  float edge = 1.0 - smoothstep(6.0, 8.5, abs(b.x));
  float yc = clamp(b.y, 0.0, L - 0.2);
  float d = length(vec2(b.x, b.y - yc));
  // a cool silver sheen round the mirror, ripples brightening it as they pass
  float rip = 0.5 + 0.5*sin(b.y*1.6 - uTime*9.0);
  vec3 light = vec3(0.75, 0.82, 0.95)*halo(d, 0.9)*(0.35 + 0.25*rip)*on + vec3(0.6, 0.7, 0.9)*halo(d, 2.0)*0.08*on;
  // glints on the flung droplets and dripping beads
  for (int k = 0; k < 10; k++){
    vec4 fl = qs_fling(float(k), uTime);
    if (fl.w <= 0.0) continue;
    light += vec3(0.85, 0.9, 1.0)*halo(length(b - fl.xy), fl.z*1.8)*0.25*fl.w;
  }
  // clash: a bright ripple ring and a splash crown
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    vec2 o = vec2(0.0, L*0.55);
    float ring = abs(length((b - o)*vec2(1.0, 0.6)) - 0.5 - 7.0*a);
    light += vec3(0.9, 0.95, 1.0)*halo(ring, 0.12)*uClash*1.3;
    light += vec3(0.9, 0.95, 1.0)*halo(abs(b.y - o.y) - 12.0*a, 0.3)*fillD(abs(b.x) - 0.9, px)*uClash;
  }
  light *= edge*(1.0 - smoothstep(L + 2.0, L + 6.0, b.y))*smoothstep(-3.0, -0.5, b.y);
  return (1.0 - exp(-light*1.1))*1.05*clamp(uIgn*1.5, 0.0, 1.0);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return vec3(0.0); }
vec4 sig_solid(vec4 o, vec2 b, float px){
  px = min(px, 0.06);
  float L = uLength;
  float ign = smoothstep(0.0, 1.5, uAge);
  float fillTo = mix(-0.5, L + 0.1, ign);                       // ignition: the metal pours up
  // ripples bulge the surface: the width wobbles where a ripple passes
  float rip = sin(b.y*1.6 - uTime*9.0)*0.5 + 0.5;
  float w = qs_w(b.y)*(1.0 + 0.12*rip);
  float cov = fillD(abs(b.x) - w, px)*step(-0.3, b.y)*fillD(b.y - min(L + 0.05, fillTo), px);
  vec4 acc = o;
  if (cov > 0.0){
    float n = b.x/max(w, 0.01);
    float slope = n + 0.25*cos(b.y*1.6 - uTime*9.0);           // ripples tilt what it reflects
    vec3 c = qs_env(slope, b.y, uTime);
    c *= 0.6 + 0.4*sqrt(max(1.0 - n*n, 0.0));
    c += vec3(1.0)*exp(-pow((b.y - mod(uTime*14.0, L + 10.0) + 4.0)/0.6, 2.0))*0.5;   // a sliding glint
    acc = vec4(mix(acc.rgb, c, cov), max(acc.a, cov));
  }
  // beads rolling up the blade, swelling at the tip and dripping off
  for (int k = 0; k < 5; k++){
    float fk = float(k);
    float per = 3.0 + fk*0.7;
    float tt = mod(uTime + fk*1.37, per)/per;
    float y = tt*(L + 1.0) - 0.5;
    float side = mod(fk, 2.0) < 0.5 ? 1.0 : -1.0;
    float rad = 0.38 + 0.12*h11(fk);
    vec2 p = vec2(side*qs_w(y)*0.9, y);
    // past the tip: it drips off and falls away in an arc
    float past = max(y - L, 0.0);
    p += vec2(side*past*1.4, past*0.3 - past*past*0.9);
    vec4 dr = qs_drop(b - p, rad*(1.0 + 0.4*smoothstep(L - 3.0, L, y)), px);
    dr.a *= (1.0 - smoothstep(L + 0.5, L + 1.0, y))*ign;
    acc = vec4(mix(acc.rgb, dr.rgb, dr.a), max(acc.a, dr.a));
  }
  // droplets flung off the blade
  for (int k = 0; k < 10; k++){
    vec4 fl = qs_fling(float(k), uTime);
    if (fl.w <= 0.0) continue;
    vec4 dr = qs_drop(b - fl.xy, fl.z, px);
    acc = vec4(mix(acc.rgb, dr.rgb, dr.a*fl.w), max(acc.a, dr.a*fl.w));
  }
  // clash: a ring of mercury drops splashing out
  if (uClash > 0.01){
    float a = 1.0 - uClash;
    for (int k = 0; k < 10; k++){
      float fk = float(k);
      float an = fk*0.628 + 0.3*h11(fk);
      vec2 p = vec2(0.0, L*0.55) + vec2(cos(an), sin(an))*(0.8 + 6.0*(1.0 - exp(-3.0*a))) - vec2(0.0, 2.0*a*a);
      vec4 dr = qs_drop(b - p, 0.3*(1.0 - 0.5*a), px);
      acc = vec4(mix(acc.rgb, dr.rgb, dr.a), max(acc.a, dr.a));
    }
  }
  acc.a *= clamp(uIgn*1.5, 0.0, 1.0);
  return acc;
}
