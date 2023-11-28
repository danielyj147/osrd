package fr.sncf.osrd.stdcm.graph

import edu.umd.cs.findbugs.annotations.SuppressFBWarnings
import fr.sncf.osrd.envelope.Envelope
import fr.sncf.osrd.envelope_sim.allowances.utils.AllowanceValue
import fr.sncf.osrd.envelope_sim.allowances.utils.AllowanceValue.FixedTime
import fr.sncf.osrd.graph.Graph
import fr.sncf.osrd.sim_infra.api.BlockInfra
import fr.sncf.osrd.sim_infra.api.RawSignalingInfra
import fr.sncf.osrd.stdcm.STDCMStep
import fr.sncf.osrd.stdcm.preprocessing.interfaces.BlockAvailabilityInterface
import fr.sncf.osrd.train.RollingStock
import fr.sncf.osrd.train.RollingStock.Comfort

/** This is the class that encodes the STDCM problem as a graph on which we can run our pathfinding implementation.
 * Most of the logic has been delegated to helper classes in this module:
 * AllowanceManager handles adding delays using allowances,
 * BacktrackingManager handles backtracking to fix speed discontinuities,
 * DelayManager handles how much delay we can and need to add to avoid conflicts,
 * STDCMEdgeBuilder handles the creation of new STDCMEdge instances  */
@SuppressFBWarnings("FE_FLOATING_POINT_EQUALITY")
class STDCMGraph(
    val rawInfra: RawSignalingInfra,
    val blockInfra: BlockInfra,
    val rollingStock: RollingStock,
    val comfort: Comfort?,
    val timeStep: Double,
    blockAvailability: BlockAvailabilityInterface,
    maxRunTime: Double,
    minScheduleTimeStart: Double,
    steps: List<STDCMStep>,
    tag: String?,
    standardAllowance: AllowanceValue?
) : Graph<STDCMNode, STDCMEdge> {
    var stdcmSimulations: STDCMSimulations = STDCMSimulations()
    val steps: List<STDCMStep>
    val delayManager: DelayManager
    val allowanceManager: AllowanceManager
    val backtrackingManager: BacktrackingManager
    val tag: String?
    val standardAllowance: AllowanceValue?

    /** Constructor  */
    init {
        this.steps = steps
        delayManager = DelayManager(minScheduleTimeStart, maxRunTime, blockAvailability, this)
        allowanceManager = AllowanceManager(this)
        backtrackingManager = BacktrackingManager(this)
        this.tag = tag
        this.standardAllowance = standardAllowance
        assert(standardAllowance !is FixedTime) { "Standard allowance cannot be a flat time for STDCM trains" }
    }

    /** Returns the speed ratio we need to apply to the envelope to follow the given standard allowance.  */
    fun getStandardAllowanceSpeedRatio(
        envelope: Envelope
    ): Double {
        if (standardAllowance == null)
            return 1.0
        val runTime = envelope.totalTime
        val distance = envelope.totalDistance
        val allowanceRatio = standardAllowance.getAllowanceRatio(runTime, distance)
        return 1 / (1 + allowanceRatio)
    }

    override fun getEdgeEnd(edge: STDCMEdge): STDCMNode {
        return edge.getEdgeEnd(this)
    }

    override fun getAdjacentEdges(node: STDCMNode): Collection<STDCMEdge> {
        return if (node.detector == null)
            STDCMEdgeBuilder.fromNode(this, node, node.locationOnBlock!!.edge).makeAllEdges()
        else {
            val res = ArrayList<STDCMEdge>()
            val neighbors = blockInfra.getBlocksStartingAtDetector(node.detector)
            for (neighbor in neighbors) {
                res.addAll(
                    STDCMEdgeBuilder.fromNode(this, node, neighbor)
                        .makeAllEdges()
                )
            }
            res
        }
    }
}