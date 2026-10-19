// Mustafar Eruption: the lava rivers of Mustafar as a blade. Lava flows down it in bright tongues
// that cool to dark glowing rock; lava fountains spit sparks off it, dark ash smoke billows up,
// flakes of ash drift down, embers fall and the air shimmers with heat. A clash flares it.
// @part Eruption sparks
// @part Ash smoke
// @part Falling ash
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
  fx += fxSparks(ro, rd, px, y0, y1, 10.0 + 10.0*sw, 4.5, 0.6, 7.7, fire)*part(0.0);
  fx += fxSmoke(ro, rd, y0, y1, 3.8, 2.0, 1.4, vec3(0.3, 0.2, 0.18))*1.3*part(1.0);
  fx += fxFlakes(ro, rd, px, y0, y1, 3.5, 12.0, 1.8, 0.2, 0.8, 1.5, 0.6, 3.6, vec3(0.5, 0.45, 0.42))*part(2.0);
  // lava fountains: molten drops thrown up off the blade, falling back in arcs
  float lta;
  if (fxNear(ro, rd, y0, y1) < 6.0){
    for (int i = 0; i < 12; i++){
      float fi = float(i);
      float cyc = t*0.45*(0.8 + 0.4*h11(fi*2.9)) + h11(fi*7.7);
      float k = floor(cyc), q = fract(cyc);
      float y = mix(y0, y1 - 6.0, h11(k*3.1 + fi));
      float side = h11(k*5.3 + fi) < 0.5 ? -1.0 : 1.0;
      float tt = q*1.6;
      vec3 p = vec3(side*(0.5 + 2.2*tt), y + 6.0*tt - 4.5*tt*tt, 0.0);
      float d = rayPt(ro, rd, p, lta);
      fx += mix(vec3(1.0, 0.85, 0.5), vec3(1.0, 0.4, 0.08), q)*(halo(d, max(0.12, px))*1.6 + halo(d, 0.5)*0.3)*(1.0 - q)*part(0.0);
    }
  }
  fx += fxSmoke(ro, rd, y0, y1, 4.0, 2.4, 8.2, vec3(0.5, 0.3, 0.22))*0.6*part(1.0);
  return fx*uFx;
}
