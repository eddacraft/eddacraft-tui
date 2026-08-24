import { Navbar } from '@/components/navbar';
import { HeroSection } from '@/components/hero-section';
import { ShippingProof } from '@/components/shipping-proof';
import { TrustGap } from '@/components/trust-gap';
import { WriteConsequence } from '@/components/write-consequence';
import { DeliveryBoundary } from '@/components/delivery-boundary';
import { DecisionModel } from '@/components/decision-model';
import { ProductStages } from '@/components/product-stages';
import { DecisionIntegrityFlywheel } from '@/components/decision-integrity-flywheel';
import { CompanyBand } from '@/components/company-band';
import { CLIFooter } from '@/components/cli-footer';

export default function Home() {
  return (
    <main className="min-h-screen bg-void">
      <Navbar />
      <HeroSection />
      <ShippingProof />
      <TrustGap />
      <WriteConsequence />
      <DeliveryBoundary />
      <DecisionModel />
      <ProductStages />
      <DecisionIntegrityFlywheel />
      <CompanyBand />
      <CLIFooter />
    </main>
  );
}
