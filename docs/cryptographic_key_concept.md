<br>  
 

### What is Ed25519
&nbsp;&nbsp;&nbsp;Ed25519 is a public-key signature scheme built out of edwards25519 curve, using the EdDSA construction. An Ed25519 public key is the encoding of the 𝑥 and 𝑦 coordinates of a point on edwards25519

&nbsp;&nbsp;&nbsp;Reference: https://ed25519.cr.yp.to/ <br>  
&nbsp;

### What is X25519
&nbsp;&nbsp;&nbsp;X25519 is an elliptic curve Diffie-Hellman key exchange using Curve25519. It allows two parties to jointly agree on a shared secret using an insecure channel. An X25519 public key is the encoding of the 𝑥 coordinate of a point on Curve25519, hence the name X25519

&nbsp;&nbsp;&nbsp;Reference: https://cryptography.io/en/3.4.3/hazmat/primitives/asymmetric/x25519.html <br>  
&nbsp;


### What is Curve25519
&nbsp;&nbsp;&nbsp;Curve25519 is an elliptic curve over the finite field 𝔽~𝑝~, where 𝑝=2<sup>255</sup>−19, whence came the 25519 part of the name. Specifically, it is the Montgomery curve 𝑦<sup>2</sup>=𝑥<sup>3</sup>+486662𝑥<sup>2</sup>+𝑥

&nbsp;&nbsp;&nbsp;Reference: https://en.wikipedia.org/wiki/Curve25519 <br>  
&nbsp;


### What is Edwards25519
&nbsp;&nbsp;&nbsp;Edwards25519 is an elliptic curve over the finite field 𝔽~𝑝~, where 𝑝=2<sup>255</sup>−19, with a different shape, the twisted Edwards shape −𝑥<sup>2</sup>+𝑦<sup>2</sup>=1−(121665/121666)𝑥<sup>2</sup>𝑦<sup>2</sup>, which admits fast computation of 𝑃+𝑄 given the 𝑥 and 𝑦 coordinates of 𝑃 and 𝑄. It is related to Curve25519 by a birational map, so most points on Curve25519 can be mapped to edwards25519 and vice versa

&nbsp;&nbsp;&nbsp;Reference: https://math.stackexchange.com/questions/1392277/point-conversion-between-twisted-edwards-and-montgomery-curves <br>  
&nbsp;

