// Mustafar Eruption: the lava rivers of Mustafar as a blade. Lava flows down it in bright tongues
// that cool to dark glowing rock; embers fall off it and the air shimmers with heat. A swing
// stirs the flow; a clash bursts a ring of fire.
// @part Eruption sparks
// @room 8
// @palette {"corona.rim_cool":[0.5,0.06,0.0],"corona.rim_hot":[1.0,0.4,0.05],"corona.inner":[1.0,0.75,0.4],"core.white":[1,0.92,0.75],"core.fringe_cool":[1.0,0.35,0.05],"core.fringe_hot":[1.0,0.65,0.3],"light.color":[1.0,0.4,0.08],"trail":[1.0,0.4,0.08]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  float n = fbm3(vec2(b.x*2.0, b.y*0.3 + uTime*0.5));
  float rock = smoothstep(0.45, 0.6, n);
  return g*mix(vec3(1.1), vec3(0.35, 0.15, 0.1), rock*0.8);
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 fire = vec3(1.0, 0.45, 0.1);
  fx += fxSparks(ro, rd, px, y0, y1, 8.0 + 10.0*sw, 4.0, 0.6, 7.7, fire);
  fx += fxShock(ro, rd, px, L*0.55, 1.0 - cl, 8.0, fire);
  return fx*uFx*part(0.0);
}
