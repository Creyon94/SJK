// Krayt Pearl: the prized pearl of a Tatooine krayt dragon as a blade. No dragon is drawn: the
// blade has a pearly sheen of overlapping scales that ripples up it and catches the light in
// rainbow glints, and fine sand drifts round it. A swing flares the sheen; a clash a pearly ring.
// @part Desert sand
// @room 8
// @palette {"corona.rim_cool":[0.55,0.5,0.4],"corona.rim_hot":[1.0,0.92,0.8],"corona.inner":[1.0,0.98,0.94],"core.white":[1,1,1],"core.fringe_cool":[0.95,0.85,0.7],"core.fringe_hot":[1.0,0.96,0.9],"light.color":[1.0,0.92,0.8],"trail":[1.0,0.92,0.8]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec2 q = vec2(b.x*3.0, b.y*0.9 - uTime*0.8);
  vec2 c = floor(q); vec2 f = fract(q) - 0.5;
  float sc = smoothstep(0.5, 0.2, length(f + vec2(0.0, 0.25)));
  vec3 sheen = hsv(fract(b.y*0.03 + uTime*0.05 + h21(c)*0.2), 0.25, 1.0);
  return g*mix(vec3(1.0), sheen, 0.5*sc)*(0.9 + 0.3*sc + 0.4*clamp(uSwing, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 sand = vec3(1.0, 0.85, 0.6);
  fx += fxMotes(ro, rd, px, y0, y1, 1.0, 4.5, 20.0, 0.05, 3.9, sand);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, vec3(1.0, 0.95, 0.88));
  return fx*uFx*part(0.0);
}
