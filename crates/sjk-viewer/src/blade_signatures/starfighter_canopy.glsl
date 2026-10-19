// Starfighter Canopy: rain on a starfighter's canopy before take-off, as a blade. Drops of water
// run down the blade, joining and racing each other, while rain streaks past outside it.
// @part Running drops
// @part Rain
// @room 8
// @palette {"corona.rim_cool":[0.2,0.3,0.42],"corona.rim_hot":[0.55,0.72,0.9],"corona.inner":[0.85,0.92,1.0],"core.white":[0.95,0.97,1.0],"core.fringe_cool":[0.5,0.65,0.85],"core.fringe_hot":[0.85,0.92,1.0],"light.color":[0.6,0.75,0.95],"trail":[0.62,0.76,0.95]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float lane = floor(b.x*8.0);
  float y = fract(b.y*0.08 + uTime*(0.25 + 0.3*h11(lane)) + h11(lane*3.7));
  float drop = pow(y, 12.0)*step(0.4, h11(lane*1.3 + 0.5));
  return g*(0.85 + 0.7*drop*part(0.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  fx += fxFall(ro, rd, px, y0, y1, 3.0, 14.0, 22.0, 1.2, 0.15, 2.4, vec3(0.75, 0.85, 1.0))*0.55*part(1.0);
  return fx*uFx;
}
