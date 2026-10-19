// Data Disc: a spinning data disc as a blade. A disc of fine glowing grooves turns round the
// blade's middle, a bright read-light sweeping round it, and soft bands run up the blade. A
// clash sends an amber ring.
// @part Data disc
// @room 8
// @palette {"corona.rim_cool":[0.5,0.18,0.04],"corona.rim_hot":[1.0,0.55,0.18],"corona.inner":[1.0,0.85,0.6],"core.white":[1.0,0.95,0.85],"core.fringe_cool":[1.0,0.5,0.15],"core.fringe_hot":[1.0,0.85,0.6],"light.color":[1.0,0.5,0.18],"trail":[1.0,0.5,0.18]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.9 + 0.12*bands(b.y, 0.1, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(1.0, 0.6, 0.25);
  float yd = L*0.62;
  vec3 h = rayAt(ro, rd, yd);
  if (h.z > 0.0){
    float r = length(h.xy);
    float ang = atan(h.y, h.x);
    float on = smoothstep(0.8, 1.0, r)*(1.0 - smoothstep(2.8, 3.1, r));
    float gr = 0.5 + 0.5*cos(r*25.0);
    float sweep = pow(0.5 + 0.5*cos(ang - t*1.5), 10.0);
    fx += c*on*(0.12 + 0.18*gr + 0.9*sweep*gr);
  }
  fx += c*halo(ringD(ro, rd, yd, 3.0), max(0.04, px))*0.6;
  return fx*uFx*part(0.0);
}
