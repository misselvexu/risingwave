package com.risingwave.scheduler;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import com.google.inject.Guice;
import com.google.inject.Injector;
import com.risingwave.node.EndPoint;
import com.risingwave.planner.rel.physical.batch.BatchPlan;
import com.risingwave.planner.rel.physical.batch.RisingWaveBatchPhyRel;
import com.risingwave.proto.plan.PlanNode;
import com.risingwave.scheduler.query.PlanFragmenter;
import com.risingwave.scheduler.query.Query;
import com.risingwave.scheduler.stage.QueryStage;
import com.risingwave.scheduler.stage.StageId;
import java.util.List;
import java.util.Set;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import org.junit.jupiter.api.DisplayName;
import org.junit.jupiter.api.Test;

public class QueryManagerTest {

  @Test
  @DisplayName("Schedule one task")
  public void testGetResultLocation()
      throws ExecutionException, InterruptedException, TimeoutException {
    Injector injector = Guice.createInjector(new TestSchedulerModule());

    QueryManager queryManager = injector.getInstance(QueryManager.class);

    QueryResultLocation resultLocation =
        queryManager.schedule(newSingleNodePlan()).get(10, TimeUnit.SECONDS);

    assertEquals(
        new EndPoint("localhost", 1234),
        resultLocation.getNodeEndPoint(),
        "Result endpoint not match!");
  }

  @Test
  public void testBreakSingleNodePlan() {
    BatchPlan plan = newSingleNodePlan();
    Query query = PlanFragmenter.planDistribution(plan);

    QueryStage rootStage = query.getQueryStageChecked(query.getRootStageId());
    PlanNode protoPlan = rootStage.getPlanInfo().getRoot().serialize();
    assertEquals(protoPlan.getNodeType(), PlanNode.PlanNodeType.CREATE_TABLE);

    List<StageId> leafStages = query.getLeafStages();
    assertEquals(leafStages.size(), 1);
    assertEquals(leafStages.get(0), query.getRootStageId());

    Set<StageId> children = query.getChildrenChecked(query.getRootStageId());
    assertEquals(children.size(), 0);

    Set<StageId> parents = query.getParentsChecked(query.getRootStageId());
    assertEquals(parents.size(), 0);
  }

  private static BatchPlan newSingleNodePlan() {
    PlanNode planNode =
        PlanNode.newBuilder().setNodeType(PlanNode.PlanNodeType.CREATE_TABLE).build();

    RisingWaveBatchPhyRel root = mock(RisingWaveBatchPhyRel.class);
    when(root.serialize()).thenReturn(planNode);
    BatchPlan batchPlan = mock(BatchPlan.class);
    when(batchPlan.getRoot()).thenReturn(root);

    return batchPlan;
  }
}
