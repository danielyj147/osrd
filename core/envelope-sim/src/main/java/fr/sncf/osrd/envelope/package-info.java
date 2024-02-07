/**
 *
 *
 * <h2>Overview</h2>
 *
 * <p>Envelopes are a framework for working with position / speed curves, with constant acceleration
 * between each point. This isn't as simple as it may seem, as acceleration works over time, and the
 * envelopes work over space.
 *
 * <ul>
 *   <li>Envelopes and envelope parts are immutable.
 *   <li>Envelopes are a sequence of envelope parts.
 *   <li>Each envelope part is a continuous space speed curve, with monotonically increasing
 *       position.
 *   <li>Each envelope part must start in space where the previous part stopped.
 *   <li>Envelope parts can be seen as a sequence of steps.
 *   <li>Each step takes a given time, moves forward in space, and has a start and an end speed.
 *   <li>The contiguous ends for the envelope parts must have the same speed for the envelope to be
 *       continuous.
 * </ul>
 *
 * <h2>Intended usage</h2>
 *
 * <p>Envelope are meant to:
 *
 * <ul>
 *   <li>Represent the most constraining speed for a vehicle.
 *   <li>Be used to calculate intermediate running times inside a simulation.
 *   <li>Store the result of simulations.
 * </ul>
 *
 * <p>Overlays can be used to add a lower speed region to an envelope. For example, an existing
 * envelope can be overlayed with a braking curve to a signal, as well as an acceleration curve
 * starting from the signal and stopping where the curve meets another envelope part.
 *
 * <h2>Features</h2>
 *
 * <ul>
 *   <li>An envelope can be built from a set of overlapping envelope parts, which are interpreted as
 *       a set of constraints: {@link fr.sncf.osrd.envelope.MRSPEnvelopeBuilder}
 *   <li>An envelope can be be overlayed with a new envelope part: {@link
 *       fr.sncf.osrd.envelope.OverlayEnvelopeBuilder}
 *   <li>Each envelope part can store arbitrary metadata: {@link fr.sncf.osrd.envelope.EnvelopeAttr}
 * </ul>
 */
package fr.sncf.osrd.envelope;
