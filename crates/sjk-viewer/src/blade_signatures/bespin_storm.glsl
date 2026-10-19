// Bespin Storm: a storm in the cloud sea of Bespin. A gale winds round the blade: streaks of wind
// light spiral up it, cloud mist streams off it and drifting motes are swept round, all calming
// toward a bright still tip. A swing whips the gale up; a clash a ring of wind.
// @part Gale
// @room 8
// @palette {"corona.rim_cool":[0.55,0.35,0.25],"corona.rim_hot":[1.0,0.75,0.55],"corona.inner":[1.0,0.93,0.85],"core.white":[1,1,1],"core.fringe_cool":[1.0,0.65,0.45],"core.fringe_hot":[1.0,0.88,0.75],"light.color":[1.0,0.75,0.55],"trail":[1.0,0.75,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.85 + 0.25*bands(b.y + b.x*4.0, 0.15, 1.5));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(1.0, 0.82, 0.65);
  fx += fxCoils(ro, rd, px, y0, L*0.85, 2.2, 0.6, 0.8, 3.0, c)*0.8;
  fx += fxMotes(ro, rd, px, y0, y1, 1.5, 5.0, 18.0, 0.12, 5.1, c);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, c);
  return fx*uFx*part(0.0);
}
