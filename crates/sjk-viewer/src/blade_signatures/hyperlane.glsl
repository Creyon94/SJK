// Hyperlane: a hyperspace lane folded into a blade. Rings of light rush down round it in a deep
// blue tunnel, starlines stream along it, two lanes of light wind round it and bands race down the
// blade. A swing brightens the tunnel; a clash a white ring.
// @part Tunnel rings
// @part Starlines
// @part Lane light
// @room 8
// @palette {"corona.rim_cool":[0.1,0.2,0.6],"corona.rim_hot":[0.45,0.65,1.0],"corona.inner":[0.85,0.92,1.0],"core.white":[1,1,1],"core.fringe_cool":[0.35,0.55,1.0],"core.fringe_hot":[0.8,0.9,1.0],"light.color":[0.4,0.6,1.0],"trail":[0.4,0.6,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.85 + 0.35*bands(b.y, 0.1, -2.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 blue = vec3(0.45, 0.65, 1.0);
  for (int i = 0; i < 5; i++){
    float q = fract(-t*0.35 + float(i)/5.0);
    float d = ringD(ro, rd, mix(y0, y1, q), 1.2 + 1.5*q);
    fx += (vec3(1.0)*halo(d, max(0.04, px))*0.8 + blue*halo(d, 0.3)*0.4)*sin(3.1416*q)*(0.8 + 0.4*sw)*part(0.0);
  }
  fx += fxFall(ro, rd, px, y0, y1, 2.0, 12.0, -20.0, 1.5, 0.0, 4.4, vec3(0.8, 0.9, 1.0))*part(1.0);
  fx += fxStreams(ro, rd, px, y0, y1, 2.0, 2.8, 0.15, 4.0, 3.2, blue)*0.7*part(2.0);
  return fx*uFx;
}
