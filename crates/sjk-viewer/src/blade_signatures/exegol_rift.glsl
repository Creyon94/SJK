// Exegol Rift: the storm-torn sky over Exegol as a blade. Cold cracks of light open and close
// along a pale blade, short violet lightning crawls down its sides, dark smoke drifts off it and
// sparks spit from the cracks; afterimages trail every swing. A clash flares it.
// @part Rift cracks
// @part Dark smoke
// @part Rift sparks
// @arcs Rift lightning
// @room 8
// @palette {"corona.rim_cool":[0.08,0.1,0.35],"corona.rim_hot":[0.5,0.65,1.0],"corona.inner":[0.06,0.06,0.18],"core.white":[0.35,0.4,0.7],"core.fringe_cool":[0.3,0.4,0.9],"core.fringe_hot":[0.65,0.75,1.0],"light.color":[0.45,0.55,1.0],"trail":[0.45,0.55,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  vec2 cell;
  float e = voro(vec2(b.x*3.0, b.y*0.35), cell);
  float open_ = smoothstep(0.6, 1.0, sin(uTime*0.8 + h21(cell)*6.2832))*part(0.0);
  return g*(0.85 + 0.5*clamp(uClash, 0.0, 1.0)) + vec3(0.8, 0.75, 1.0)*halo(e, 0.05)*open_*lvl(g)*1.6;
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 v = vec3(0.7, 0.55, 1.0);
  fx += fxSmoke(ro, rd, y0, y1, 3.6, 1.2, 3.8, vec3(0.2, 0.15, 0.3))*1.4*part(1.0);
  fx += fxSparks(ro, rd, px, y0, y1, 8.0 + 8.0*sw, 3.0, 1.3, 6.6, v)*part(2.0);
  return fx*uFx;
}
