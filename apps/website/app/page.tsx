import { Navbar } from '@/components/navbar';
import { HeroSection } from '@/components/hero-section';
import { ShippingProof } from '@/components/shipping-proof';
import { TrustGap } from '@/components/trust-gap';
import { WriteConsequence } from '@/components/write-consequence';
import { DecisionIntegrityFlywheel } from '@/components/decision-integrity-flywheel';
import { ProductStages } from '@/components/product-stages';
import { DecisionModel } from '@/components/decision-model';
import { DeliveryBoundary } from '@/components/delivery-boundary';
import { CompanyBand } from '@/components/company-band';
import { CLIFooter } from '@/components/cli-footer';

// Reading order follows the approved public-site spec:
// customer problem → working product → larger system → one delivery boundary → company.
export default function Home() {
  return (
    <main className="min-h-screen bg-void">
      <Navbar />
      <HeroSection />
      <ShippingProof />
      <TrustGap />
      <WriteConsequence />
      <DecisionIntegrityFlywheel />
      <ProductStages />
      <DecisionModel />
      <DeliveryBoundary />
      <CompanyBand />
      <CLIFooter />
    </main>
  );
}
