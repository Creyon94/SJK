// Bespin Storm: a storm in the clouds round Cloud City. A blade with wind bands slanting across it
// in the cloud's colour, soft sunset clouds of rose and gold billowing round it, gale streaks
// racing past and flashes of storm lightning inside the clouds. A clash flares it.
// @part Cloud City clouds
// @part Gale
// @arcs Storm lightning
// @room 8
// @palette {"corona.rim_cool":[0.55,0.35,0.25],"corona.rim_hot":[1.0,0.75,0.55],"corona.inner":[1.0,0.93,0.85],"core.white":[1,1,1],"core.fringe_cool":[1.0,0.65,0.45],"core.fringe_hot":[1.0,0.88,0.75],"light.color":[1.0,0.75,0.55],"trail":[1.0,0.75,0.55]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.8 + 0.35*bands(b.y + b.x*4.0, 0.15, 1.5));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 rose = vec3(1.0, 0.65, 0.5), gold = vec3(1.0, 0.82, 0.55);
  fx += fxSmoke(ro, rd, y0, y1, 4.2, 1.2, 1.1, rose)*part(0.0);
  fx += fxSmoke(ro, rd, y0 + 4.0, y1, 3, 0.8, 7.5, gold)*0.6*part(0.0);
  float s;
  if (fxNear(ro, rd, y0, y1) < 6.0){
    for (int i = 0; i < 14; i++){
      float fi = float(i);
      float f = fract(t*0.5*(0.7 + 0.6*h11(fi*2.3)) + h11(fi*5.9));
      float y = mix(y0, y1, h11(fi*1.7 + floor(t*0.5 + h11(fi*5.9))));
      float x = mix(-5.0, 5.0, f);
      float d = raySeg(ro, rd, vec3(x, y, 0.0), vec3(x + 1.8, y - 0.3, 0.0), s);
      fx += vec3(1.0, 0.95, 0.9)*(halo(d, max(0.03, px))*0.6 + halo(d, 0.2)*0.1)*sin(3.1416*f)*part(1.0);
    }
  }
  return fx*uFx;
}
