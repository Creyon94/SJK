// Christophsis Crystal: the crystal fields of Christophsis as a blade. Clear violet crystals grow out
// of the blade one by one, slanting up, glint for a while and sink back into it; the blade's
// glow ripples faintly. A clash flares it.
// @part Crystals
// @room 8
// @palette {"corona.rim_cool":[0.3,0.1,0.55],"corona.rim_hot":[0.75,0.45,1.0],"corona.inner":[0.92,0.8,1.0],"core.white":[0.97,0.92,1.0],"core.fringe_cool":[0.65,0.35,1.0],"core.fringe_hot":[0.9,0.78,1.0],"light.color":[0.7,0.4,1.0],"trail":[0.68,0.4,1.0]}

vec3 sig_glow(vec3 g, vec2 b, float r, float px){
  return g*(0.92 + 0.12*bands(b.y, 0.08, 0.5));
}
vec3 sig_core(vec3 c, vec2 b, float px){ return c; }
// the effects, flat round the blade as Sol's are
vec3 sig_vol(vec3 ro, vec3 rd, float px){
  float L = uLength, t = uTime, sw = clamp(uSwing, 0.0, 1.0), cl = clamp(uClash, 0.0, 1.0);
  float y0 = L*0.2, y1 = L*0.97;
  vec3 fx = vec3(0.0);
  vec3 c = vec3(0.8, 0.6, 1.0);
  float s;
  if (fxNear(ro, rd, y0, y1) < 3.5){
    for (int i = 0; i < 10; i++){
      float fi = float(i);
      float cyc = t*0.1 + h11(fi*4.3);
      float k = floor(cyc), q = fract(cyc);
      float grow = smoothstep(0.0, 0.5, q)*(1.0 - smoothstep(0.85, 1.0, q));
      if (grow < 0.01) continue;
      float y = mix(y0, y1 - 2.0, h11(fi*2.7 + k*3.1));
      float side = h11(fi*9.1 + k) < 0.5 ? -1.0 : 1.0;
      vec3 a = vec3(side*0.4, y, 0.0);
      vec3 tip = a + vec3(side*(1.0 + 1.2*h11(fi + k*5.0)), 0.6 + 1.2*h11(fi*3.0 + k), 0.0)*grow;
      float d = raySeg(ro, rd, a, tip, s);
      float w = mix(0.22, 0.03, s)*grow;
      float gl = pow(0.5 + 0.5*sin(t*1.7 + fi*2.3), 12.0);
      fx += (vec3(1.0)*halo(d, max(w*0.5, px))*(0.8 + gl) + c*halo(d, max(w*1.8, px))*0.8 + c*halo(d, 0.5)*0.1)*grow;
    }
  }
  return fx*uFx*part(0.0);
}
