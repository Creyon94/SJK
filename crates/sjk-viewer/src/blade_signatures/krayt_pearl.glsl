// Krayt Pearl: the pearl of a great krayt dragon, as a blade. Pearl scales shimmer pink and blue
// down the blade; grains of desert sand stream past it on the wind, a veil of sand dust curls off it
// and the air shimmers with Tatooine's heat. A swing brightens the pearl; a clash a pale ring.
// @part Pearl sheen
// @part Desert sand
// @part Sand veil
// @room 8
// @palette {"corona.rim_cool":[0.55,0.5,0.4],"corona.rim_hot":[1.0,0.92,0.8],"corona.inner":[1.0,0.98,0.94],"core.white":[1,1,1],"core.fringe_cool":[0.95,0.85,0.7],"core.fringe_hot":[1.0,0.96,0.9],"light.color":[1.0,0.92,0.8],"trail":[1.0,0.92,0.8]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec2 q = vec2(b.x*3.0, b.y*0.9 - uTime*0.8);
  vec2 c = floor(q); vec2 f = fract(q) - 0.5;
  float sc = smoothstep(0.5, 0.2, length(f + vec2(0.0, 0.25)))*part(0.0);
  vec3 sheen = hsv(fract(b.y*0.03 + uTime*0.05 + h21(c)*0.2), 0.3, 1.0);
  return g*mix(vec3(1.0), sheen, 0.55*sc)*(0.9 + 0.35*sc + 0.4*clamp(uSwing, 0.0, 1.0));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 sand = vec3(1.0, 0.85, 0.6);
  fx += fxFall(ro, rd, px, y0, y1, 3.5, 16.0, 6.0, 0.4, 1.4, 6.2, sand)*0.8*part(1.0);
  fx += fxSmoke(ro, rd, y0, y1, 3.4, 1.0, 6.6, sand*0.7)*part(2.0);
  return fx*uFx;
}
